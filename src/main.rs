use std::thread;
use std::time::Duration;
use sysinfo::{Disks, System};

fn main() {
    let mut sys = System::new_all();

    println!("Starting real-time system monitor (Press Ctrl+C to exit)...\n");

    loop {
        // Clear screen (cross-platform terminal clear)
        print!("{esc}[2J{esc}[1;1H", esc = 27 as char);

        // CPU metrics require time delta between refreshes to calculate percentage accurately
        sys.refresh_cpu_usage();
        thread::sleep(Duration::from_millis(1000));
        sys.refresh_cpu_usage();

        // --- 1. CPU METRICS ---
        let global_cpu = sys.global_cpu_usage();
        println!("==================================================");
        println!(" CPU USAGE");
        println!("==================================================");
        println!("Overall CPU Load: {:.2}%", global_cpu);
        println!("Logical Cores:   {}\n", sys.cpus().len());

        for (i, cpu) in sys.cpus().iter().enumerate() {
            println!(
                "  Core {:<2} [{:<12}]: {:>5.1}% | {} MHz",
                i,
                cpu.name(),
                cpu.cpu_usage(),
                cpu.frequency()
            );
        }

        // --- 2. RAM & SWAP METRICS ---
        sys.refresh_memory();
        let bytes_to_gb = 1_073_741_824.0;

        let total_ram = sys.total_memory() as f64 / bytes_to_gb;
        let used_ram = sys.used_memory() as f64 / bytes_to_gb;
        let free_ram = sys.free_memory() as f64 / bytes_to_gb;
        let ram_pct = (sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0;

        let total_swap = sys.total_swap() as f64 / bytes_to_gb;
        let used_swap = sys.used_swap() as f64 / bytes_to_gb;

        println!("\n==================================================");
        println!(" RAM & SWAP USAGE");
        println!("==================================================");
        println!(
            "RAM Usage:  {:.2} GB / {:.2} GB ({:.1}%) [Free: {:.2} GB]",
            used_ram, total_ram, ram_pct, free_ram
        );
        println!(
            "Swap Usage: {:.2} GB / {:.2} GB",
            used_swap, total_swap
        );

        // --- 3. DISK METRICS ---
        let disks = Disks::new_with_refreshed_list();
        println!("\n==================================================");
        println!(" DISK STORAGE");
        println!("==================================================");

        for disk in &disks {
            let total_disk = disk.total_space() as f64 / bytes_to_gb;
            let avail_disk = disk.available_space() as f64 / bytes_to_gb;
            let used_disk = total_disk - avail_disk;
            let disk_pct = if total_disk > 0.0 {
                (used_disk / total_disk) * 100.0
            } else {
                0.0
            };

            println!(
                "Mount: {:<12} | FS: {:<5} | Usage: {:.1}/{:.1} GB ({:.1}%)",
                disk.mount_point().to_string_lossy(),
                disk.file_system().to_string_lossy(),
                used_disk,
                total_disk,
                disk_pct
            );
        }

        println!("\nPolling interval: 1s...");
    }
}