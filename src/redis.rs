use std::{any::Any, collections::{HashMap, LinkedList}, fs::OpenOptions, io::Write, process::{exit, id}, ptr::null_mut, sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard}};
use libc::{close, dup2, fclose, fopen, fork, fprintf, getpid, off_t, open, pid_t, setsid, signal, FILE, O_RDWR, SIGHUP, SIGPIPE, SIG_IGN, STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO};
use once_cell::sync::Lazy;
use crate::{ae::{create_file_event, create_time_event, el::Mask, handler::{accept_handler, server_cron}}, anet::tcp_server, util::{log, oom, timestamp, LogLevel}};
use self::{client::RedisClient, db::RedisDB, signal::setup_sig_segv_action};

pub mod config;
pub mod client;
pub mod cmd;
pub mod obj;
pub mod skiplist;
pub mod db;
pub mod signal;
pub mod vm;
pub mod aof;
pub mod rdb;


/// 
/// Redis Server.
///  


pub const IO_BUF_LEN: usize = 1024;
static ONE_GB: i32 = 1024 * 1024 * 1024;
static MAX_IDLE_TIME: i32 = 60 * 5;             // default client timeout
static DEFAULT_DBNUM: i32 = 16;
static SERVER_PORT: u16 = 6379;

// Hashes related defaults
static HASH_MAX_ZIPMAP_ENTRIES: usize = 64;
static HASH_MAX_ZIPMAP_VALUE: usize = 512;


/// Redis Server state.
/// 
pub static SERVER: Lazy<Arc<RwLock<RedisServer>>> = Lazy::new(|| { Arc::new(RwLock::new(RedisServer::new())) });
pub fn server_read() -> RwLockReadGuard<'static, RedisServer> {
    SERVER.read().unwrap()
}
pub fn server_write() -> RwLockWriteGuard<'static, RedisServer> {
    SERVER.write().unwrap()
}

pub struct RedisServer {
    port: u16,
    fd: i32,
    dbs: Vec<Arc<RwLock<RedisDB>>>,
    sharing_pool: HashMap<String, String>,      // Pool used for object sharing
    sharing_pool_size: u32,
    dirty: u128,                                // changes to DB from the last save
    slaves: LinkedList<Arc<RwLock<RedisClient>>>,
    monitors: LinkedList<RedisClient>,
    cron_loops: i32,                                            // number of times the cron function run
    obj_free_list: LinkedList<Arc<dyn Any + Sync + Send>>,      // A list of freed objects to avoid malloc()
    last_save: u64,                                             // Unix time of last save succeeded (in seconds)
    // Fields used only for stats
    stat_starttime: u64,                        // server start time (in seconds)
    stat_numcommands: u128,                     // number of processed commands
    stat_numconnections: u128,                  // number of connections received
    // Configuration
    verbosity: LogLevel,
    glue_output_buf: bool,
    max_idle_time: i32,
    dbnum: i32,
    daemonize: bool,
    append_only: bool,
    append_fsync: AppendFsync,
    append_writer: Option<Arc<dyn Write + Sync + Send>>,
    last_fsync: u64,
    append_fd: i32,
    append_sel_db: i32,
    pid_file: String,
    bg_save_child_pid: pid_t,
    bg_rewrite_child_pid: pid_t,
    bg_rewrite_buf: String,                     // buffer taken by parent during oppend only rewrite
    save_params: Vec<SaveParam>,
    log_file: String,
    bind_addr: String,
    db_filename: String,
    append_filename: String,
    require_pass: String,
    share_objects: bool,
    rdb_compression: bool,
    // Replication related
    is_slave: bool,
    master_auth: String,
    master_host: String,
    master_port: u16,
    master: Option<Arc<RedisClient>>,       // client that is master for this slave
    repl_state: ReplState,

    max_clients: u32,
    max_memory: u128,
    blpop_blocked_clients: u32,
    vm_blocked_clients: u32,
    // Virtual memory configuration
    vm_enabled: bool,
    vm_swap_file: String,
    vm_page_size: off_t,
    vm_pages: off_t,
    vm_max_memory: u128,
    
    
    // Hashes config
    hash_max_zipmap_entries: usize,
    hash_max_zipmap_value: usize,

    // Virtual memory state
    unix_time: u64,                                 // Unix time sampled every second

    vm_max_threads: i32,                            // Max number of I/O threads running at the same time

    devnull: Option<Arc<dyn Write + Sync + Send>>,

    // Virtual memory I/O threads stuff
    // An I/O thread process an element taken from the io_jobs queue and
    // put the result of the operation in the io_done list. While the
    // job is being processed, it's put on io_processing queue.

    io_ready_clients: LinkedList<Arc<RwLock<RedisClient>>>,     // Clients ready to be unblocked. All keys loaded
}
impl RedisServer {
    pub fn new() -> RedisServer {
        let save_params = vec![
            SaveParam { seconds: 60 * 60, changes: 1 },             // save after 1 hour and 1 change
            SaveParam { seconds: 300, changes: 100 },               // save after 5 minutes and 100 changes
            SaveParam { seconds: 60, changes: 10000 },              // save after 1 minute and 10000 changes
        ];
        RedisServer { 
            port: SERVER_PORT, 
            fd: -1,
            dbs: Vec::with_capacity(DEFAULT_DBNUM as usize),
            sharing_pool: HashMap::new(),
            dirty: 0,
            slaves: LinkedList::new(),
            monitors: LinkedList::new(),
            cron_loops: 0,
            obj_free_list: LinkedList::new(),
            last_save: timestamp().as_secs(),
            stat_starttime: timestamp().as_secs(),
            stat_numcommands: 0,
            stat_numconnections: 0,
            verbosity: LogLevel::Verbose,
            max_idle_time: MAX_IDLE_TIME,
            dbnum: DEFAULT_DBNUM,
            save_params,
            log_file: String::new(),                       // "" = log on standard output
            bind_addr: String::new(),
            glue_output_buf: true,
            daemonize: false,
            append_only: false,
            append_fsync: AppendFsync::Always,
            append_writer: None,
            last_fsync: timestamp().as_secs(),
            append_fd: -1,
            append_sel_db: -1,                  // Make sure the first time will not match
            pid_file: "/var/run/redis.pid".to_string(),
            bg_save_child_pid: -1,
            bg_rewrite_child_pid: -1,
            bg_rewrite_buf: String::new(),
            db_filename: "dump.rdb".to_string(),
            append_filename: "appendonly.aof".to_string(),
            require_pass: String::new(),
            share_objects: false,
            rdb_compression: true,
            sharing_pool_size: 1024,
            max_clients: 0,
            blpop_blocked_clients: 0,
            max_memory: 0,
            vm_enabled: false,
            vm_swap_file: "/tmp/redis-%p.vm".to_string(),
            vm_page_size: 256,                  // 256 bytes per page
            vm_pages: 1024 * 1024 * 100,        // 104 millions of pages
            vm_max_memory: 1024 * 1024 * 1024,  // 1 GB of RAM
            vm_max_threads: 4,
            vm_blocked_clients: 0,
            hash_max_zipmap_entries: HASH_MAX_ZIPMAP_ENTRIES,
            hash_max_zipmap_value: HASH_MAX_ZIPMAP_VALUE,
            unix_time: timestamp().as_secs(),

            // Replication related
            is_slave: false,
            master_auth: String::new(),
            master_host: String::new(),
            master_port: 6379,
            master: None,
            repl_state: ReplState::None,
            devnull: None,
            
            io_ready_clients: LinkedList::new(),
        }
    }

    pub fn init_server(&mut self) {
        unsafe {
            // ignore handler
            signal(SIGHUP, SIG_IGN);
            signal(SIGPIPE, SIG_IGN);
            setup_sig_segv_action();
        }

        match OpenOptions::new().write(true).open("/dev/null") {
            Ok(f) => { self.devnull = Some(Arc::new(f)); },
            Err(e) => {
                log(LogLevel::Warning, &format!("Can't open /dev/null: {}", e));
                exit(1);
            },
        }

        match tcp_server(self.port, &self.bind_addr) {
            Ok(fd) => { self.fd = fd; },
            Err(e) => {
                log(LogLevel::Warning, &format!("Opening TCP port: {}", e));
                exit(1);
            },
        }

        for i in 0..self.dbnum {
            self.dbs.push(Arc::new(RwLock::new(RedisDB::new(self.vm_enabled, i))));
        }

        create_time_event(1, Arc::new(server_cron), None, None);
        match create_file_event(self.fd, Mask::Readable, Arc::new(accept_handler)) {
            Ok(_) => {},
            Err(e) => { oom(&e); },    // TODO: is it appropriate to call oom?
        }

        /* if self.append_only {
            match OpenOptions::new().write(true).append(true).create(true).open(self.append_filename) {
                Ok(f) => { self.append_writer = Some(Box::new(f)); },
                Err(e) => {
                    self.log(LogLevel::Warning, &format!("Can't open the append-only file: {}", e));
                    exit(1);
                },
            }
        } */

        // if self.vm_enabled { self.init_vm(); }
    }

    pub fn daemonize(&self) {
        let mut _fd = -1;
        let mut _fp: *mut FILE = null_mut();
        unsafe {
            if fork() != 0 { exit(0); }     // parent exits
            setsid();                               // create a new session
    
            // Every output goes to /dev/null. If Redis is daemonized but
            // the 'logfile' is set to 'stdout' in the configuration file
            // it will not log at all.
            _fd = open("/dev/null".as_ptr() as *const i8, O_RDWR, 0);
            if _fd != -1 {
                dup2(_fd, STDIN_FILENO);
                dup2(_fd, STDOUT_FILENO);
                dup2(_fd, STDERR_FILENO);
                if _fd > STDERR_FILENO { close(_fd); }
            }
    
            // Try to write the pid file
            _fp = fopen(self.pid_file.as_ptr() as *const i8, "w".as_ptr() as *const i8);
            if !_fp.is_null() {
                fprintf(_fp, "%d\n".as_ptr() as *const i8, getpid());
                fclose(_fp);
            }
        }
    }

    /// This function gets called when 'maxmemory' is set on the config file to limit
    /// the max memory used by the server, and we are out of memory.
    /// This function will try to, in order:
    /// 
    /// - Free objects from the free list
    /// - Try to remove keys with an EXPIRE set
    /// 
    /// It is not possible to free enough memory to reach used-memory < maxmemory
    /// the server will start refusing commands that will enlarge even more the
    /// memory usage.
    pub fn free_memory_if_needed(&mut self) {
        // TODO
        log(LogLevel::Warning, "free memory if needed!!!");
    }

    pub fn clear(&mut self) -> u128 {
        let mut removed = 0u128;
        for db in &self.dbs {
            let mut db_w = db.write().unwrap();
            removed += db_w.dict.len() as u128;
            db_w.dict.clear();
            db_w.expires.clear();
        }
        removed
    }

    
    pub fn reset_server_save_params(&mut self) {
        self.save_params.clear();
    }
    fn append_server_save_params(&mut self, seconds: u128, changes: i32) {
        self.save_params.push(SaveParam { seconds, changes });
    }

    pub fn log_file(&self) -> &str {
        &self.log_file
    }
    pub fn verbosity(&self) -> &LogLevel {
        &self.verbosity
    }
    pub fn vm_enabled(&self) -> bool {
        self.vm_enabled
    }
    pub fn cron_loops(&self) -> i32 {
        self.cron_loops
    }
    pub fn set_cron_loops(&mut self, c: i32) {
        self.cron_loops = c;
    }
    pub fn set_unix_time(&mut self, t: u64) {
        self.unix_time = t;
    }
    pub fn dbnum(&self) -> i32 {
        self.dbnum
    }
    pub fn dbs(&self) -> &Vec<Arc<RwLock<RedisDB>>> {
        &self.dbs
    }
    pub fn bg_save_child_pid(&self) -> i32 {
        self.bg_save_child_pid
    }
    pub fn io_ready_clients(&self) -> &LinkedList<Arc<RwLock<RedisClient>>> {
        &self.io_ready_clients
    }
    pub fn max_clients(&self) -> u32 {
        self.max_clients
    }
    pub fn stat_numconnections(&self) -> u128 {
        self.stat_numconnections
    }
    pub fn set_stat_numconnections(&mut self, s: u128) {
        self.stat_numconnections = s;
    }
    pub fn slaves(&self) -> &LinkedList<Arc<RwLock<RedisClient>>> {
        &self.slaves
    }
    pub fn sharing_pool(&self) -> &HashMap<String, String> {
        &self.sharing_pool
    }
    pub fn is_daemonize(&self) -> bool {
        self.daemonize
    }
    pub fn append_only(&self) -> bool {
        self.append_only
    }
    pub fn append_filename(&self) -> &str {
        &self.append_filename
    }
    pub fn db_filename(&self) -> &str {
        &self.db_filename
    }
    pub fn port(&self) -> u16 {
        self.port
    }

    #[cfg(target_os = "linux")]
    pub fn linux_overcommit_memory_warning(&self) {
        if self.linux_overcommit_memory_value() == 0 {
            log(LogLevel::Warning, "WARNING overcommit_memory is set to 0! Background save may fail under low condition memory. To fix this issue add 'vm.overcommit_memory = 1' to /etc/sysctl.conf and then reboot or run the command 'sysctl vm.overcommit_memory=1' for this to take effect.");
        }
    }
    #[cfg(target_os = "linux")]
    fn linux_overcommit_memory_value(&self) -> i32 {
        use std::io::{BufRead, BufReader, Read};

        let mut _reader: Option<Box<dyn Read>> = None;
        match OpenOptions::new().read(true).open("/proc/sys/vm/overcommit_memory") {
            Ok(f) => { _reader = Some(Box::new(f)); },
            Err(e) => {
                log(LogLevel::Warning, &format!("Can't open '/proc/sys/vm/overcommit_memory' file: {}", e));
                return -1;
            },
        }
        let mut buf = String::new();
        match BufReader::new(_reader.unwrap()).read_line(&mut buf) {
            Ok(_) => {
                match buf.trim().parse() {
                    Ok(r) => r,
                    Err(e) => {
                        log(LogLevel::Warning, &format!("Parsing '{}' as i32 failed: {}", buf, e));
                        -1
                    },
                }
            },
            Err(e) => {
                log(LogLevel::Warning, &format!("Reading '/proc/sys/vm/overcommit_memory' file failed: {}", e));
                -1
            },
        }
    }
}


#[derive(PartialEq)]
enum ReplState {
    // Slave replication state - slave side
    None,       // No active replication
    Connect,    // Must connect to master
    Connected,  // Connected to master
    // Slave replication state - from the point of view of master
    // Note that in SEND_BULK and ONLINE state the slave receives new updates
    // in its output queue. In the WAIT_BGSAVE state instead the server is waiting
    // to start the next background saving in order to send updates to it.
    WaitBgSaveStart,        // master waits bgsave to start feeding it
    WaitBgSaveEnd,          // master waits bgsave to start bulk DB transmission
    SendBulk,               // master is sending the bulk DB
    Online,                 // bulk DB already transmitted, receive updates
}


struct SaveParam {
    seconds: u128,
    changes: i32,
}


enum AppendFsync {
    No,
    Always,
    EverySec,
}


static REDIS_VERSION: &str = "1.3.7";
pub fn print_logo() {
    log(LogLevel::Notice, &format!("                _._                                                  "));
    log(LogLevel::Notice, &format!("           _.-``__ ''-._                                             "));
    log(LogLevel::Notice, &format!("      _.-``    `.  `_.  ''-._           Rudis {}", REDIS_VERSION));
    log(LogLevel::Notice, &format!("  .-`` .-```.  ```\\/    _.,_ ''-._                                   "));
    log(LogLevel::Notice, &format!(" (    '      ,       .-`  | `,    )     Re-implementation in Rust!"));
    log(LogLevel::Notice, &format!(" |`-._`-...-` __...-.``-._|'` _.-'|     Port: {}", server_read().port()));
    log(LogLevel::Notice, &format!(" |    `-._   `._    /     _.-'    |     PID: {}", id()));
    log(LogLevel::Notice, &format!("  `-._    `-._  `-./  _.-'    _.-'                                   "));
    log(LogLevel::Notice, &format!(" |`-._`-._    `-.__.-'    _.-'_.-'|                                  "));
    log(LogLevel::Notice, &format!(" |    `-._`-._        _.-'_.-'    |           http://redis.io        "));
    log(LogLevel::Notice, &format!("  `-._    `-._`-.__.-'_.-'    _.-'                                   "));
    log(LogLevel::Notice, &format!(" |`-._`-._    `-.__.-'    _.-'_.-'|                                  "));
    log(LogLevel::Notice, &format!(" |    `-._`-._        _.-'_.-'    |                                  "));
    log(LogLevel::Notice, &format!("  `-._    `-._`-.__.-'_.-'    _.-'                                   "));
    log(LogLevel::Notice, &format!("      `-._    `-.__.-'    _.-'                                       "));
    log(LogLevel::Notice, &format!("          `-._        _.-'                                           "));
    log(LogLevel::Notice, &format!("              `-.__.-'                                               "));
}


#[cfg(test)]
mod tests {
    use std::io::{BufRead, Cursor};

    #[test]
    fn char_test() {
        assert!('\t'.is_whitespace());
        assert!('\r'.is_whitespace());
        assert!('\n'.is_whitespace());
        assert!(' '.is_whitespace());
    }

    #[test]
    fn cfg_file_line_test() {
        let text = "\n\n\n\n".to_string();
        let cursor = Cursor::new(text);
        let lines: Vec<String> = cursor.lines().map(|l| l.unwrap()).collect();
        assert_eq!(lines.len(), 4);
    }
}

