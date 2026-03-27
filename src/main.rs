use adb_client::ADBDeviceExt;
use adb_client::server::ADBServer;
use adb_client::server_device::ADBServerDevice;
use chrono::{Datelike, Duration as ChronoDuration, Local, Weekday};
use rand::prelude::*;
use std::env;
use std::thread;
use std::time::Duration;

fn get_current_time() -> chrono::DateTime<Local> {
    let mut server = ADBServer::default();
    if let Ok(mut device) = server.get_device() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        if device
            .shell_command(
                &"date +%s",
                Some(&mut stdout as &mut dyn std::io::Write),
                Some(&mut stderr as &mut dyn std::io::Write),
            )
            .is_ok()
        {
            if let Ok(output) = String::from_utf8(stdout) {
                if let Ok(timestamp) = output.trim().parse::<i64>() {
                    if let Some(dt) = chrono::DateTime::from_timestamp(timestamp, 0) {
                        let device_time = dt.with_timezone(&Local);
                        let local_time = Local::now();
                        let diff = device_time
                            .signed_duration_since(local_time)
                            .num_seconds()
                            .abs();
                        if diff > 60 {
                            println!(
                                "Warning: System time differs from device time by {} seconds. Using device time.",
                                diff
                            );
                        }
                        return device_time;
                    }
                }
            }
        }
    }
    println!("Warning: Could not get time from device, using system time.");
    Local::now()
}

fn main() {
    println!("Starting Mox ADB Automation Scheduler...");

    // Check for "now" or "immediate" argument
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && (args[1] == "now" || args[1] == "immediate") {
        println!("Immediate execution requested.");
        if let Err(e) = run_automation() {
            eprintln!("Automation failed: {}", e);
        } else {
            println!("Automation completed successfully.");
        }
        // Exit after immediate execution if that's desired behavior?
        // User probably wants it to run once then maybe continue schedule or just run once.
        // Usually CLI tools that take "now" run once and exit, or run once and then loop.
        // Let's assume run once and loop for now, as it's a scheduler.
        // Wait a bit to prevent double triggering if the schedule logic aligns perfectly with now.
        thread::sleep(Duration::from_secs(60));
    }

    loop {
        let now = get_current_time();
        let next_run = get_next_run_time(now);

        let wait_duration = next_run
            .signed_duration_since(now)
            .to_std()
            .unwrap_or(Duration::from_secs(0));

        println!("Current time: {}", now.format("%Y-%m-%d %H:%M:%S"));
        println!(
            "Next run scheduled for: {}",
            next_run.format("%Y-%m-%d %H:%M:%S")
        );
        println!("Waiting for {:?}...", wait_duration);

        thread::sleep(wait_duration);

        println!("Executing scheduled task...");
        if let Err(e) = run_automation() {
            eprintln!("Automation failed: {}", e);
        } else {
            println!("Automation completed successfully.");
        }

        // Sleep a bit to avoid immediate re-triggering if logic is slightly off,
        // though get_next_run_time should handle future times correctly.
        thread::sleep(Duration::from_secs(60));
    }
}

fn get_next_run_time(now: chrono::DateTime<Local>) -> chrono::DateTime<Local> {
    let mut candidate = now;

    // Check if we can run today
    // Morning slot: 8:20 + random(0..5) min
    // Evening slot: 18:40 + random(0..5) min

    // We need to find the next valid slot that is in the future.
    // Since we want to schedule a specific time, let's look ahead day by day.

    loop {
        // Only run on weekdays (Mon-Fri)
        let weekday = candidate.weekday();
        if weekday != Weekday::Sat && weekday != Weekday::Sun {
            // Check morning slot
            let morning_start = candidate
                .date_naive()
                .and_hms_opt(8, 20, 0)
                .unwrap()
                .and_local_timezone(Local)
                .unwrap();
            // Generate random delay for this slot
            let delay_min = rand::rng().random_range(0..=5);
            let morning_run = morning_start + ChronoDuration::minutes(delay_min);

            if morning_run > now {
                return morning_run;
            }

            // Check evening slot
            let evening_start = candidate
                .date_naive()
                .and_hms_opt(17, 40, 0)
                .unwrap()
                .and_local_timezone(Local)
                .unwrap();
            // Generate random delay for this slot
            let delay_min = rand::rng().random_range(0..=5);
            let evening_run = evening_start + ChronoDuration::minutes(delay_min);

            if evening_run > now {
                return evening_run;
            }
        }

        // Move to next day
        candidate += ChronoDuration::days(1);
        // Reset time to start of day to ensure we catch morning slots of next days
        candidate = candidate
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap();
    }
}

fn run_automation() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting unlock sequence...");

    // Initialize ADB server connection
    let mut server = ADBServer::default();

    // Get the first connected device
    let mut device = server.get_device()?;

    println!(
        "Connected to device: {}",
        device.identifier.as_deref().unwrap_or("Unknown")
    );

    // 1. Wake up the device
    println!("Running: input keyevent KEYCODE_WAKEUP");
    device.shell_command(
        &["input", "keyevent", "KEYCODE_WAKEUP"].join(" "),
        Some(&mut std::io::stdout()),
        Some(&mut std::io::stderr()),
    )?;

    // 2. Sleep 1 second
    thread::sleep(Duration::from_secs(1));

    // 3. Swipe up (unlock screen)
    println!("Running: input swipe 360 1300 360 400 300");
    device.shell_command(
        &["input", "swipe", "360", "1300", "360", "400", "300"].join(" "),
        Some(&mut std::io::stdout()),
        Some(&mut std::io::stderr()),
    )?;

    // 4. Sleep 1 second
    thread::sleep(Duration::from_secs(1));

    // 5. Continuous Swipe Z pattern
    println!("Running: Continuous Z pattern");
    let points = vec![(110, 1000), (600, 1000), (110, 1400), (600, 1400)];

    // Attempt to perform continuous swipe
    if let Err(e) = perform_continuous_swipe(&mut device, &points, 200) {
        eprintln!(
            "Failed to perform continuous swipe: {}. Falling back to discrete swipes.",
            e
        );

        // Fallback to discrete swipes if continuous fails
        for i in 0..points.len() - 1 {
            let (x1, y1) = points[i];
            let (x2, y2) = points[i + 1];
            println!("Fallback: swipe {} {} {} {} 200", x1, y1, x2, y2);
            device.shell_command(
                &format!("input swipe {} {} {} {} 200", x1, y1, x2, y2),
                Some(&mut std::io::stdout()),
                Some(&mut std::io::stderr()),
            )?;
        }
    }

    println!("Unlock sequence completed.");

    // 6. Launch DingTalk
    println!("Launching DingTalk...");
    // Wait for unlock animation to complete
    thread::sleep(Duration::from_secs(2));

    device.shell_command(
        &"am start -n com.alibaba.android.rimet/.biz.LaunchHomeActivity",
        Some(&mut std::io::stdout()),
        Some(&mut std::io::stderr()),
    )?;

    println!("DingTalk launch command sent.");

    // 7. Wait 30 seconds
    println!("Waiting for 30 seconds before locking screen...");
    thread::sleep(Duration::from_secs(30));

    // 8. Lock screen
    println!("Locking screen: input keyevent KEYCODE_POWER");
    device.shell_command(
        &["input", "keyevent", "KEYCODE_POWER"].join(" "),
        Some(&mut std::io::stdout()),
        Some(&mut std::io::stderr()),
    )?;
    
    println!("Screen locked.");

    Ok(())
}

fn perform_continuous_swipe(
    device: &mut ADBServerDevice,
    points: &[(i32, i32)],
    _duration_ms: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    // Use input motionevent for continuous swipe without lifting finger
    // We chain commands in a single shell script to minimize latency.
    // We interpolate points to simulate a smooth drag.

    let mut script = String::new();
    script.push_str("#!/system/bin/sh\n");

    // Number of steps per segment for interpolation
    // A segment length of ~500 pixels (110->600)
    // 20 steps => ~25 pixels per step, which is reasonable for touch
    let steps = 20;

    if let Some(first) = points.first() {
        script.push_str(&format!("input motionevent DOWN {} {}\n", first.0, first.1));
    }

    for i in 0..points.len() - 1 {
        let (x1, y1) = points[i];
        let (x2, y2) = points[i + 1];

        for step in 1..=steps {
            let t = step as f32 / steps as f32;
            let x = (x1 as f32 + (x2 - x1) as f32 * t) as i32;
            let y = (y1 as f32 + (y2 - y1) as f32 * t) as i32;
            script.push_str(&format!("input motionevent MOVE {} {}\n", x, y));
        }
    }

    if let Some(last) = points.last() {
        script.push_str(&format!("input motionevent UP {} {}\n", last.0, last.1));
    }

    // Upload script
    let remote_path = "/data/local/tmp/swipe.sh";

    let mut reader = std::io::Cursor::new(script.into_bytes());
    device.push(&mut reader, remote_path)?;

    // Make executable
    device.shell_command(
        &format!("chmod 755 {}", remote_path),
        Some(&mut std::io::stdout()),
        Some(&mut std::io::stderr()),
    )?;

    // Run script
    println!("Executing continuous swipe script (motionevent)...");
    device.shell_command(
        &format!("sh {}", remote_path),
        Some(&mut std::io::stdout()),
        Some(&mut std::io::stderr()),
    )?;

    // Cleanup
    device.shell_command(
        &format!("rm {}", remote_path),
        Some(&mut std::io::stdout()),
        Some(&mut std::io::stderr()),
    )?;

    Ok(())
}
