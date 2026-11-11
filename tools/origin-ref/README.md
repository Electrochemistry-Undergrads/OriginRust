# Reference numbers from Origin

`run-labtalk.ps1` runs a LabTalk script in a temporary Origin instance through COM automation and
prints the values of chosen variables. The numeric parity tests in `crates/calc` compare our
results with numbers produced this way, on inputs that we generated ourselves.

- It needs a computer with Origin installed. Tests never call it; they use the numbers that were
  copied into the test fixtures, with the tolerance written next to them.
- The scripts in this folder are our own. They only call Origin's public scripting commands.
- One Origin instance at a time: do not run two copies of the script in parallel.

Example (PowerShell):

```
.\run-labtalk.ps1 -Script integ-parabola.labtalk.txt -Vars area
```
