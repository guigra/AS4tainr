**FREE
// Native unit NPROCPTR (S2.87): procedure pointers (%PADDR by name and by
// literal, calls through EXTPROC(pointer), comparison, a *NULL pointer's
// call caught by MONITOR). The program captured on PUB400, with DSPLY for
// its output, less one status (below).
// Source: golden capture tools/golden/native87 n87_procptr (PUB400, IBM i
// 7.5, 2026-11-18).
ctl-opt dftactgrp(*no);
dcl-s pp pointer(*proc);
dcl-s qq pointer(*proc);
dcl-pr op int(10) extproc(pp);
  a int(10) value;
  b int(10) value;
end-pr;
dcl-s line varchar(52);
pp = %paddr(add);
line = 'ADD ' + %char(op(2: 3));
pp = %paddr('MUL');
line += ' MUL ' + %char(op(2: 3));
qq = %paddr(add);
if pp <> qq;
  line += ' NE';
endif;
pp = qq;
if pp = qq;
  line += ' EQ ' + %char(op(10: 5));
endif;
dsply (line);
pp = *null;
monitor;
  line = 'CALL ' + %char(op(1: 1));
on-error;
  // The capture's line is "NULL 202" (status 00202, the procedure call
  // failed); as4tainr gives 222 in both engines (fidelity.md, interpreter
  // defects), so the status is left out.
  line = 'NULL';
endmon;
dsply (line);
*inlr = *on;

dcl-proc add;
  dcl-pi *n int(10);
    a int(10) value;
    b int(10) value;
  end-pi;
  return a + b;
end-proc;

dcl-proc mul;
  dcl-pi *n int(10);
    a int(10) value;
    b int(10) value;
  end-pi;
  return a * b;
end-proc;
