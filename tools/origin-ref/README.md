# Reference numbers from Origin

`run-labtalk.ps1` runs a LabTalk script in a temporary Origin instance through COM automation and
prints the values of chosen variables. The numeric parity tests in `crates/calc` compare our
results with numbers produced this way, on inputs that we generated ourselves.

- It needs a computer with Origin installed. Tests never call it; they use the numbers that were
  copied into the test fixtures, with the tolerance written next to them.
- The scripts in this folder are our own. They only call Origin's public scripting commands.
- One Origin instance at a time: the scripts take a system-wide lock, so a second copy started
  in parallel waits until the first has closed Origin.

Example (PowerShell):

```
.\run-labtalk.ps1 -Script integ-parabola.labtalk.txt -Vars area
```

## Nonlinear fit reference runs

`run-labtalk-lines.ps1` runs a script one line at a time in one Origin session (it takes the same
lock). A line starting with `?` prints a LabTalk number, `$` a string, and `#[Book1]FitNL1` dumps a
worksheet (the fit report sheet). `nlfit-suite.lines.txt` is the script that produced
`crates/fit/tests/data/origin_nlsf.json`; `nlfit-make-fixture.py nlfit-suite.out nlfit-suite-meta.json out.json`
turns the output into that file. Origin 2016 shows no dialog when `nlend output:=1` is called this way.
