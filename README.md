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
(cr) ((3f34d37...)) kamil@kekPC ~/trunk/src/scripts $ pwrtest -b=92 -a=/home/kamil/trunk/src/third_party/autotest/files --board=caroline --ip=10.0.0.85 -o="./pwrtest" --tests=power_WebGL,power_WebGL
powered off
charging... 89 90 90 90 90 90 90 91 91 91 91 91 91 91 done
powered on
running test power_WebGL…
battery: 89%
powered off
charging... 89 89 89 90 90 90 90 90 90 91 91 91 91 91 91 91 done
powered on
running test power_WebGL…
battery: 89%
(cr) ((3f34d37...)) kamil@kekPC ~/trunk/src/scripts $ ls ./pwrtest
test_no_1__power_WebGL__10.0.0.85  test_no_2__power_WebGL__10.0.0.85
```

The program assumes that:
- the DUT is turned on and left on login screen
- the power button isn't locked as pressed (e.g. via `dut-control pwr_button:press`)
