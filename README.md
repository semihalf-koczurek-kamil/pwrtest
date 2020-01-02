# pwrtest

```
$ pwrtest --help
pwrtest 1.0
Kamil Koczurek <kek@semihalf.com>
Tests power usage on chromebooks.

USAGE:
    pwrtest --autotest_dir <PATH> --battery_min <%LEVEL> --board <BOARD NAME> --ip <DUT IP> --out_dir <OUT DIR> --tests <TESTS>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -a, --autotest_dir <PATH>     Autotest directory
    -b, --battery_min <%LEVEL>    Minimum battery level before running a test, must range from 0 to 100
        --board <BOARD NAME>      Name of the tested board, eg. caroline
        --ip <DUT IP>             IP of the Device Under Test
    -o, --out_dir <OUT DIR>       directory to save logs
        --tests <TESTS>           comma separated test names, e.g. 'power_Display,power_Idle'
```

Example:
```
(cr) ((3f34d37...)) kamil@kekPC ~/trunk/src/scripts $ pwrtest -f=80 -t=85 -a=/home/kamil/trunk/src/third_party/autotest/files --board=caroline --ip=192.168.0.162 -o="./pwrtest" --tests="power_WebGL,power_WebGL"
powered off
below 80%! charging from 79 to 85...
███████████████████████████████████████████████████░░░░░░░░░░░░ 84/100
powered on
running test power_WebGL... took: 4.98m, 
running test power_WebGL... took: 4.40m, 
=================================
total time: 27.33m
=================================
```

Building prerequesites:
* git
* cargo

Building:
```
git clone https://github.com/semihalf-koczurek-kamil/pwrtest
cd pwrtest
cargo build
```

To use, copy the binary (`target/debug/pwrtest`) to `<chromiumos_source>/chroot/usr/bin/`.

Note: The program assumes that the power button isn't locked as pressed (e.g. via `dut-control pwr_button:press`).
