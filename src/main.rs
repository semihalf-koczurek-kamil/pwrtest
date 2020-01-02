use std::{
	time::{Duration, SystemTime},
	str::from_utf8,
	thread::sleep,
	net::IpAddr,
	io::Write,
};

macro_rules! dut_control {
    ($($arg:tt)*) => ({
		::std::process::Command::new("dut-control")
			.args(&[ $($arg)* ])
    })
}

macro_rules! test_that {
    ($($arg:tt)*) => ({
		::std::process::Command::new("test_that")
			.args(&[ $($arg)* ])
    })
}

struct Conf {
	tests: Vec<String>,
	charge_from: i32,
	charge_to: i32,
	autotest_dir: String,
	out_dir: String,
	board: String,
	ip: String,
}

fn time_to_string(dur: Duration) -> String {
	let secs = dur.as_secs() as f32;
	if secs < 60.0 {
		format!("{:.2}s", secs)
	} else if secs < 3600.0 {
		format!("{:.2}m", secs / 60.0)
	} else {
		format!("{:.2}h", secs / 3600.0)
	}
}

fn battery_validator(arg: String) -> Result<(), String> {
	let err_msg = "not an integer between 0 and 100".to_owned();
	let parsed = arg.parse::<i32>();

	match parsed {
		Err(_) => Err(err_msg),
		Ok(n) => if n > 100 || n < 0 {
			Err(err_msg)
		} else {
			Ok(())
		}
	}
}

fn path_validator(arg: String) -> Result<(), String> {
	let err_msg = "invalid path".to_owned();
	if std::path::Path::new(&arg).exists() {
		Ok(())
	} else {
		Err(err_msg)
	}
}

fn ip_validator(arg: String) -> Result<(), String> {
	let err_msg = "not a valid ip address".to_owned();
	match arg.parse::<IpAddr>() {
		Ok(_) => Ok(()),
		Err(_) => Err(err_msg)
	}
}

fn get_config() -> Conf {
	use clap::{App, Arg};

	let matches = App::new("pwrtest")
        .version("1.0")
        .author("Kamil Koczurek <kek@semihalf.com>")
        .about("Tests power usage on chromebooks.")
        .arg(Arg::with_name("charge_from")
			.short("f")
			.long("charge_from")
			.value_name("%FROM")
			.help("If the battery level falls below this value between test, the DUT is powered off and charged")
			.validator(battery_validator)
			.required(true)
		)
		.arg(Arg::with_name("charge_to")
			.short("t")
			.long("charge_to")
			.value_name("%TO")
			.help("the DUT is charged to this value (default=%FROM)")
			.validator(battery_validator)
		)
		.arg(Arg::with_name("autotest_dir")
			.short("a")
			.long("autotest_dir")
			.value_name("PATH")
			.help("Autotest directory")
			.validator(path_validator)
			.required(true)
		)
		.arg(Arg::with_name("board")
			.long("board")
			.value_name("BOARD NAME")
			.help("Name of the tested board, eg. caroline")
			.required(true)
		)
		.arg(Arg::with_name("ip")
			.long("ip")
			.value_name("DUT IP")
			.help("IP of the Device Under Test")
			.validator(ip_validator)
			.required(true)
		)
		.arg(Arg::with_name("out_dir")
			.short("o")
			.long("out_dir")
			.value_name("OUT DIR")
			.help("directory to save logs")
			.validator(path_validator)
			.required(true)
		)
		.arg(Arg::with_name("tests")
			.long("tests")
			.value_name("TESTS")
			.help("comma separated test names, e.g. 'power_Display,power_Idle'")
			.value_delimiter(",")
			.required(true)
		)
		.get_matches();

	/* All arguments are prevalidated, so unwraps below are all safe */
	let tests = matches.values_of("tests").unwrap().map(&str::to_owned).collect();
	let charge_from = matches.value_of("charge_from").unwrap().parse().unwrap();
	let charge_to = matches.value_of("charge_to").map(|s| s.parse::<i32>().unwrap()).unwrap_or(charge_from);
	let autotest_dir = matches.value_of("autotest_dir").unwrap().to_owned();
	let out_dir = matches.value_of("out_dir").unwrap().to_owned();
	let board = matches.value_of("board").unwrap().to_owned();
	let ip = matches.value_of("ip").unwrap().to_owned();

	let conf = Conf {
		tests,
		charge_from, charge_to,
		autotest_dir, out_dir,
		board, ip,
	};

	if conf.charge_from > conf.charge_to {
		println!("ERR: charge_from is greater than charge_to");
		std::process::exit(1);
	}

	conf
}

fn pwr_button(enable: bool) {
	let pwr_button = if enable {
		"pwr_button:press"
	} else {
		"pwr_button:release"
	};

	dut_control![pwr_button]
		.spawn()
		.unwrap();
}

fn powerstate_try() -> Option<String> {
	let stdout = dut_control!["ec_system_powerstate"]
		.output()
		.unwrap()
		.stdout;

	let stdout = from_utf8(&stdout)
		.expect("dut-control output is not valid utf8")
		.to_owned();

	let parts: Vec<&str> = stdout.split(':').collect();

	if 1 >= parts.len() {
		return None;
	}

	let state_with_newline = parts[1];
	let res = state_with_newline
		[..state_with_newline.len() - 1] //drop the newline
		.to_owned();

	Some(res)
}

fn powerstate() -> String {
	let mut res = powerstate_try();
	while res.is_none() {
		println!("failed to retrieve ec_system_powerstate, retrying in 10s...");
		sleep(Duration::from_secs(10));
		res = powerstate_try();
	}

	res.unwrap()
}


fn powered_on() -> bool {
	const OFF: u8 = b'G';
	const ON: u8 = b'S';

	let powerstate = powerstate();

	if powerstate.bytes().next() == Some(ON) {
		true
	} else if powerstate.bytes().next() == Some(OFF) {
		false
	} else {
		panic!("invalid powerstate: {}", powerstate);
	}
}

fn poweroff() {
	if powered_on() {
		pwr_button(true);
		sleep(Duration::from_secs(3));
		pwr_button(false);
		sleep(Duration::from_secs(10));
		println!("powered off");
	}
}

fn poweron() {
	if !powered_on() {
		pwr_button(true);
		sleep(Duration::from_secs(1));
		pwr_button(false);
		sleep(Duration::from_secs(10));
		println!("powered on");
	}
}

fn wallpower(enable: bool) {
	let servo_v4_role = if enable {
		"servo_v4_role:src"
	} else {
		"servo_v4_role:snk"
	};

	dut_control![servo_v4_role]
		.spawn()
		.expect("couldn't run dut-control to set wallpower, are you in cros_sdk chroot?");
}

fn battery_pct_try() -> Option<i32> {
	let stdout = dut_control!["battery_charge_percent"]
		.output()
		.expect("failed to probe for battery charge level, are you in cros_sdk chroot?")
		.stdout;

	let stdout = from_utf8(&stdout)
		.expect("dut-control output is not valid utf8")
		.to_owned();

	let parts: Vec<&str> = stdout.split(':').collect();

	if 1 >= parts.len() {
		return None;
	}

	let pct_text = parts[1]; // with \n
	let maybe_pct = pct_text[..pct_text.len() - 1] // drop \n
		.parse();

	if let Ok(pct) = maybe_pct {
		Some(pct)
	} else {
		None
	}
}

fn battery_pct() -> i32 {
	let mut res = battery_pct_try();
	while res.is_none() {
		println!("failed to retrieve battery_charge_percent, retrying in 30s...");
		sleep(Duration::from_secs(30));
		res = battery_pct_try();
	}

	res.unwrap()
}

fn charge(from: i32, to: i32) {
	let pct = battery_pct();
	if pct < from {
		wallpower(true);

		poweroff();
		println!("below {}%! charging from {} to {}...", from, pct, to);

		let bar = indicatif::ProgressBar::new(100);
		bar.inc(pct as u64);

		let mut old_pct = pct;
		let mut pct = pct;
		while pct < to {
			if old_pct != pct {
				bar.inc((pct - old_pct) as u64);
				old_pct = pct;
			}

			sleep(Duration::from_secs(10));
			pct = battery_pct();
		}

		bar.finish_at_current_pos();
		poweron();
	}

	wallpower(false);
}

fn run_test(board: &str, autotest_dir: &str, ip: &str, test: &str) -> String {
	let out = test_that![
			&format!("--board={}", board),
			&format!("--autotest_dir={}", autotest_dir),
			ip,
			test,
		]
		.output()
		.expect(&format!("failed to run test {} on {}", test, ip));

	from_utf8(&out.stdout)
		.expect("test output is not valid utf8")
		.to_owned()
}

fn main() {
	let beginning = SystemTime::now();
	let config = get_config();
	let mut test_n = 1;

	for test in &config.tests {
		charge(config.charge_from, config.charge_to);

		print!("running test {}... ", test);
		std::io::stdout().flush().unwrap();

		let test_beginning = SystemTime::now();

		let out = run_test(&config.board, &config.autotest_dir, &config.ip, test);
		let filename = format!("{}/test_no_{}__{}__{}", config.out_dir, test_n, test, &config.ip);
		let write_err_msg = format!("failed to save result of test {} to {}", test, &filename);

		std::fs::write(
			&filename,
			out
		).expect(&write_err_msg);

		println!("took: {}, ", time_to_string(test_beginning.elapsed().unwrap()));

		test_n += 1;
	}

	println!("=================================");
	println!("total time: {}", time_to_string(beginning.elapsed().unwrap()));
	println!("=================================");
}
