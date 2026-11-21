**FREE
// Native unit NFLTDEC (S2.203): %DEC, %DECH, %INT and %INTH of floats,
// the program captured on PUB400 as GN203FD with DSPLY for its output,
// less its last statement, %INTH(pow(2 : 0.5)): 1 on IBM i, an error in
// the interpreter (fidelity.md, "%INTH of a float with many digits").
// Source: golden capture tools/golden/native203 n203_floatdec (PUB400,
// IBM i 7.5, 2026-11-21).
ctl-opt dftactgrp(*no);
dcl-pr QXXZTOD float(8) extproc('QXXZTOD');
  source pointer value;
  digits int(10) value;
  frac int(10) value;
end-pr;
dcl-pr pow float(8) extproc('pow');
  x float(8) value;
  y float(8) value;
end-pr;
dcl-s zoned zoned(7:2) inz(-12.55);
dcl-s f float(8) inz(2.675);
dcl-s g float(8) inz(-7.5);
dcl-s line varchar(200);

line = %char(%dech(QXXZTOD(%addr(zoned) : 7 : 2) : 7 : 1));
line += ' ' + %char(%dec(f : 7 : 2));
line += ' ' + %char(%dech(g : 5 : 0));
line += ' ' + %char(%int(g));
dsply (line);
*inlr = *on;
