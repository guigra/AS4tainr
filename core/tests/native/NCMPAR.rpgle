**FREE
// Native unit NCMPAR (S2.145): arithmetic in conditions, the program
// captured on PUB400 as GN145CA with DSPLY for its output.
// Source: golden capture tools/golden/native145 n145_cmparith (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-s a packed(5:2) inz(1.25);
dcl-s b packed(5:2) inz(1.20);
dcl-s n int(10) inz(7);
dcl-s line varchar(80);

if a * b = 1.5;
  line += 'a';
endif;
if n / 3 > 2.33;
  line += 'b';
endif;
// The capture's `10 / 3 = 3.3333333333333333333333333333` test is left
// out: IBM i and the interpreter differ there (fidelity.md, "A quotient
// compared in a condition").
if a + b - 2.45 = 0;
  line += 'd';
endif;
if n * 2 in %list(3 : 14);
  line += 'e';
endif;
if (a - b) * 10 < 0.6;
  line += 'f';
endif;
dow n - 5 > 0;
  n -= 1;
enddo;
line += %char(n);
dsply (line);
*inlr = *on;
