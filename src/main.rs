use std::{
	time::{Duration, SystemTime},
	process::Command,
	str::from_utf8,
	thread::sleep,
	net::IpAddr,
	io::Write,
};

struct Conf {
	tests: Vec<String>,
	charge_from: i32,
	charge_to: i32,
	autotest_dir: String,
	out_dir: String,
	board: String,
	ip: String,
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
			.help("If the DUT is charged to this value")
			.validator(battery_validator)
			.required(true)
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
			.required(true)
		)
		.get_matches();

	/* All arguments are prevalidated, so unwraps below are all safe */
	let conf = Conf {
		tests: matches.value_of("tests").unwrap().split(",").map(|r| r.to_owned()).collect(),
		charge_from: matches.value_of("charge_from").unwrap().parse().unwrap(),
		charge_to: matches.value_of("charge_to").unwrap().parse().unwrap(),
		autotest_dir: matches.value_of("autotest_dir").unwrap().to_owned(),
		out_dir: matches.value_of("out_dir").unwrap().to_owned(),
		board: matches.value_of("board").unwrap().to_owned(),
		ip: matches.value_of("ip").unwrap().to_owned(),
	};

	if conf.charge_from > conf.charge_to {
		println!("ERR: charge_from is greater than charge_to");
		std::process::exit(1);
	}

	conf
}

fn pwr_button(enable: bool) {
	Command::new("dut-control")
		.args(&[format!(
			"pwr_button:{}",
			if enable { "press" } else { "release" }
		)])
		.spawn()
		.unwrap();
}

fn poweroff() {
	pwr_button(true);
	sleep(Duration::from_secs(3));
	pwr_button(false);
	sleep(Duration::from_secs(10));
	println!("powered off");
}

fn poweron() {
	pwr_button(true);
	sleep(Duration::from_secs(1));
	pwr_button(false);
	sleep(Duration::from_secs(10));
	println!("powered on");
}

fn wallpower(enable: bool) {
	Command::new("dut-control")
		.args(&[format!(
			"servo_v4_role:{}",
			if enable { "src" } else { "snk" }
		)])
		.spawn()
		.expect("couldn't run dut-control to set wallpower, are you in cros_sdk chroot?");
}

fn battery_pct_try() -> Option<i32> {
	let stdout = Command::new("dut-control")
		.args(&["battery_charge_percent"])
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
		print!("E ");
		std::io::stdout().flush().unwrap();
		sleep(Duration::from_secs(30));
		res = battery_pct_try();
	}

	res.unwrap()
}

fn charge(from: i32, to: i32) {
	wallpower(false);
	if battery_pct() < from {
		poweroff();
		print!("below {}%! charging... ", from);
		std::io::stdout().flush().unwrap();

		while battery_pct() < to {
			print!("{} ", battery_pct());
			std::io::stdout().flush().unwrap();
			wallpower(true);
			sleep(Duration::from_secs(30));
		}

		println!("done");
		wallpower(false);
		poweron();
	}
}

fn run_test(board: &str, autotest_dir: &str, ip: &str, test: &str) -> String {
	let out = Command::new("test_that")
		.args(&[
			&format!("--board={}", board),
			&format!("--autotest_dir={}", autotest_dir),
			ip,
			test,
		])
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

		println!("running test {}…", test);
		let test_beginning = SystemTime::now();

		let out = run_test(&config.board, &config.autotest_dir, &config.ip, test);
		let filename = format!("{}/test_no_{}__{}__{}", config.out_dir, test_n, test, &config.ip);
		let write_err_msg = format!("failed to save result of test {} to {}", test, &filename);

		std::fs::write(
			&filename,
			out
		).expect(&write_err_msg);

		println!("{} time: {}mins", test, test_beginning.elapsed().unwrap().as_secs() as f32 / 60.0);
		println!("battery: {}%", battery_pct());

		test_n += 1;
	}

	println!("=================================");
	println!("total time: {}mins", beginning.elapsed().unwrap().as_secs() as f32 / 60.0);
	println!("=================================");
}
