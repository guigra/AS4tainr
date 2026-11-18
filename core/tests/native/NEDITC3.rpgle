**FREE
// Native unit NEDITC3 (S2.81): %EDITC with *ASTFILL, *CURSYM and a
// literal as its third argument. The program captured on PUB400, with
// DSPLY for its output, less one part of a line (below).
// Source: golden capture tools/golden/native81 n81_editc (PUB400, IBM i
// 7.5, 2026-11-18).
ctl-opt dftactgrp(*no);
dcl-s v packed(7:2) dim(3);
dcl-s i int(10);
dcl-s x packed(7:2);
v(1) = 1234.5;
v(2) = -42;
v(3) = 0;
for i = 1 to 3;
  x = v(i);
  dsply ('A|' + %editc(x: 'J': *astfill) + '|' + %editc(x: '1': *astfill) + '|');
  dsply ('C|' + %editc(x: 'J': *cursym) + '|' + %editc(x: 'A': *cursym) + '|');
  // For zero, %EDITC(x: 'K': '+') is all blanks on IBM i (the capture's
  // "L|     +.00|           |"); as4tainr puts the '+' after them in both
  // engines (fidelity.md, interpreter defects), so that part is left out.
  if i < 3;
    dsply ('L|' + %editc(x: '3': '+') + '|' + %editc(x: 'K': '+') + '|');
  else;
    dsply ('L|' + %editc(x: '3': '+') + '|');
  endif;
endfor;
*inlr = *on;
