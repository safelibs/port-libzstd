Programs and scripts for automated testing of Zstandard
=======================================================

This directory contains the following programs and scripts:
- `datagen` : Synthetic and parametrable data generator, for tests
- `fullbench`  : Precisely measure speed for each zstd inner functions
- `fuzzer`  : Test tool, to check zstd integrity on target platform
- `paramgrill` : parameter tester for zstd
- `test-zstd-speed.py` : script for testing zstd speed difference between commits
- `test-zstd-versions.py` : compatibility test between zstd versions stored on Github (v0.1+)
- `zstreamtest` : Fuzzer test tool for zstd streaming API
- `external_matchfinder` : Public match-parameter and prefix/streaming coverage
- `legacy` : Test tool to test decoding of legacy zstd frames
- `decodecorpus` : Tool to generate valid Zstandard frames, for verifying decoder implementations


#### `test-zstd-versions.py` - script for testing zstd interoperability between versions

This script creates `versionsTest` directory to which zstd repository is cloned.
Then all tagged (released) versions of zstd are compiled.
In the following step interoperability between zstd versions is checked.

#### `automated-benchmarking.py` - script for benchmarking zstd prs to dev

This script benchmarks facebook:dev and changes from pull requests made to zstd and compares
them against facebook:dev to detect regressions. This script currently runs on a dedicated
desktop machine for every pull request that is made to the zstd repo but can also
be run on any machine via the command line interface.

There are three modes of usage for this script: fastmode will just run a minimal single
build comparison (between facebook:dev and facebook:release), onetime will pull all the current
pull requests from the zstd repo and compare facebook:dev to all of them once, continuous
will continuously get pull requests from the zstd repo and run benchmarks against facebook:dev.

```
Example usage: python automated_benchmarking.py
```

```
usage: automated_benchmarking.py [-h] [--directory DIRECTORY]
                                 [--levels LEVELS] [--iterations ITERATIONS]
                                 [--emails EMAILS] [--frequency FREQUENCY]
                                 [--mode MODE] [--dict DICT]

optional arguments:
  -h, --help            show this help message and exit
  --directory DIRECTORY
                        directory with files to benchmark
  --levels LEVELS       levels to test e.g. ('1,2,3')
  --iterations ITERATIONS
                        number of benchmark iterations to run
  --emails EMAILS       email addresses of people who will be alerted upon
                        regression. Only for continuous mode
  --frequency FREQUENCY
                        specifies the number of seconds to wait before each
                        successive check for new PRs in continuous mode
  --mode MODE           'fastmode', 'onetime', 'current', or 'continuous' (see
                        README.md for details)
  --dict DICT           filename of dictionary to use (when set, this
                        dictionary will be used to compress the files provided
                        inside --directory)
```

#### `test-zstd-speed.py` - script for testing zstd speed difference between commits

DEPRECATED

This script creates `speedTest` directory to which zstd repository is cloned.
Then it compiles all branches of zstd and performs a speed benchmark for a given list of files (the `testFileNames` parameter).
After `sleepTime` (an optional parameter, default 300 seconds) seconds the script checks repository for new commits.
If a new commit is found it is compiled and a speed benchmark for this commit is performed.
The results of the speed benchmark are compared to the previous results.
If compression or decompression speed for one of zstd levels is lower than `lowerLimit` (an optional parameter, default 0.98) the speed benchmark is restarted.
If second results are also lower than `lowerLimit` the warning e-mail is sent to recipients from the list (the `emails` parameter).

Additional remarks:
- To be sure that speed results are accurate the script should be run on a "stable" target system with no other jobs running in parallel
- Using the script with virtual machines can lead to large variations of speed results
- The speed benchmark is not performed until computers' load average is lower than `maxLoadAvg` (an optional parameter, default 0.75)
- The script sends e-mails using `mutt`; if `mutt` is not available it sends e-mails without attachments using `mail`; if both are not available it only prints a warning


The example usage with two test files, one e-mail address, and with an additional message:
```
./test-zstd-speed.py "silesia.tar calgary.tar" "email@gmail.com" --message "tested on my laptop" --sleepTime 60
```

To run the script in background please use:
```
nohup ./test-zstd-speed.py testFileNames emails &
```

The full list of parameters:
```
positional arguments:
  testFileNames         file names list for speed benchmark
  emails                list of e-mail addresses to send warnings

optional arguments:
  -h, --help            show this help message and exit
  --message MESSAGE     attach an additional message to e-mail
  --lowerLimit LOWERLIMIT
                        send email if speed is lower than given limit
  --maxLoadAvg MAXLOADAVG
                        maximum load average to start testing
  --lastCLevel LASTCLEVEL
                        last compression level for testing
  --sleepTime SLEEPTIME
                        frequency of repository checking in seconds
```

#### `decodecorpus` - tool to generate Zstandard frames for decoder testing
Command line tool to generate test .zst files.

This tool will generate .zst files with checksums,
as well as optionally output the corresponding correct uncompressed data for
extra verification.

Example:
```
./decodecorpus -ptestfiles -otestfiles -n10000 -s5
```
will generate 10,000 sample .zst files using a seed of 5 in the `testfiles` directory,
with the zstd checksum field set,
as well as the 10,000 original files for more detailed comparison of decompression results.

```
./decodecorpus -t -T1mn
```
will choose a random seed, and for 1 minute,
generate random test frames and ensure that the
zstd library correctly decompresses them in both simple and streaming modes.

`decodecorpus` only generates complete public Zstandard frames. Private raw-block
generation options such as `--gen-blocks` and `--max-block-size-log` are rejected
instead of being silently accepted.

#### `external_matchfinder` - public match-parameter coverage

`external_matchfinder` validates the public match-finding surface without relying on
sequence-producer internals. It checks public bounds/error handling for match-related
compression parameters, runs a strategy matrix round-trip suite, verifies prefix-reference
compression wins on repeated data, and exercises long-distance matching through the
streaming API.

#### `paramgrill` - public-API tester for stable compression parameters

`paramgrill` now keeps the stable public `--zstd=` / `--optimize=` surface, runs a
built-in suite of public parameter profiles by default, and validates each successful
configuration through imported frame-size helpers plus a full round-trip.

Arguments
```
 -i# or -i #     : number of passes over the built-in profile suite (default: 1)
 -s# or -s #     : input size in bytes; accepts K/M/G suffixes (default: 256K)
 -v              : print one line per successful profile or optimizer candidate
 --zstd=...      : run one explicit stable-parameter configuration
 --optimize=...  : search over the selected public parameters
 --display=...   : choose which parameters appear in the emitted --zstd= line
 -h              : display help
```
