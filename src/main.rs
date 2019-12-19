use std::{process::Command, str::from_utf8, thread::sleep, time::Duration, io::Write};

struct Conf {
	tests: Vec<String>,
	battery_min: i32,
	autotest_dir: String,
	out_dir: String,
	board: String,
	ip: String,
}

fn get_config() -> Conf {
	use clap::{App, Arg};

	let matches = App::new("pwrtest")
        .version("1.0")
        .author("Kamil Koczurek <kek@semihalf.com>")
        .about("Tests power usage on chromebooks.")
        .arg(Arg::with_name("battery_min")
			.short("b")
			.long("battery_min")
			.value_name("%LEVEL")
			.help("Minimum battery level before running a test, must range from 0 to 100")
			.validator(|s| s.parse::<i32>()
				.map_err(|_| "argument is not a valid integer".to_owned())
				.map(|_| ())
			)
			.required(true)
		)
		.arg(Arg::with_name("autotest_dir")
			.short("a")
			.long("autotest_dir")
			.value_name("PATH")
			.help("Autotest directory")
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
			.required(true)
		)
		.arg(Arg::with_name("out_dir")
			.short("o")
			.long("out_dir")
			.value_name("OUT DIR")
			.help("directory to save logs")
			.required(true)
		)
		.arg(Arg::with_name("tests")
			.long("tests")
			.value_name("TESTS")
			.help("comma separated test names, e.g. 'power_Display,power_Idle'")
			.required(true)
		)
        .get_matches();

	Conf {
		tests: matches.value_of("tests").unwrap().split(",").map(|r| r.to_owned()).collect(),
		battery_min: matches.value_of("battery_min").unwrap().parse().unwrap(),
		autotest_dir: matches.value_of("autotest_dir").unwrap().to_owned(),
		out_dir: matches.value_of("out_dir").unwrap().to_owned(),
		board: matches.value_of("board").unwrap().to_owned(),
		ip: matches.value_of("ip").unwrap().to_owned(),
	}
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
		.unwrap();
}

fn battery_pct_try() -> Option<i32> {
	let stdout = Command::new("dut-control")
		.args(&["battery_charge_percent"])
		.output()
		.expect("failed to probe for battery charge level")
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

fn charge_to(value: i32) {
	wallpower(false);
	if battery_pct() < value {
		poweroff();
		print!("charging... ");
		std::io::stdout().flush().unwrap();

		while battery_pct() < value {
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
	let config = get_config();
	let mut test_n = 1;

	for test in &config.tests {
		charge_to(config.battery_min);

		println!("running test {}…", test);

		let out = run_test(&config.board, &config.autotest_dir, &config.ip, test);
		let filename = format!("{}/test_no_{}__{}__{}", config.out_dir, test_n, test, &config.ip);
		let write_err_msg = format!("failed to save result of test {} to {}", test, &filename);

		std::fs::write(
			&filename,
			out
		).expect(&write_err_msg);

		println!("battery: {}%", battery_pct());

		test_n += 1;
	}
}
