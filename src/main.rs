use std::{env, process::exit, sync::Arc, time::Instant};
use rredis::redis::{before_sleep, log::LogLevel, server_read, server_write, REDIS_VERSION, SERVER};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 2 {
        server_write().reset_server_save_params();
        server_write().load_server_config(&args[1]);
    } else if args.len() > 2 {
        eprintln!("Usage: ./redis-server [/path/to/redis.conf]");
        exit(1);
    } else {
        server_read().log(LogLevel::Warning, "Warning: no config file specified, using the default config. In order to specify a config file use 'redis-server /path/to/redis.conf'");
    }
    if server_read().is_daemonize() {
        server_read().daemonize();
    }

    server_write().init_server();
    server_read().log(LogLevel::Notice, &format!("Server started, Redis version {}", REDIS_VERSION));

    #[cfg(target_os = "linux")]
    server_read().linux_overcommit_memory_warning();

    /* let start = Instant::now();
    if server_read().append_only() {
        if let Ok(_) = server_read().load_append_only_file() {
            server_read().log(LogLevel::Notice, &format!("DB loaded from append only file: {} seconds", start.elapsed().as_secs()));
        }
    } else {
        if let Ok(_) = server_read().rdb_load() {
            server_read().log(LogLevel::Notice, &format!("DB loaded from disk: {} seconds", start.elapsed().as_secs()));
        }
    } */

    server_read().log(LogLevel::Notice, &format!("The server is now ready to accept connections on port {}", server_read().port()));
    server_write().set_before_sleep_proc(Some(Arc::new(before_sleep)));
    server_write().main();
}
