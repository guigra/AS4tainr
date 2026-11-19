**FREE
// Native unit NFLTCMP (S2.109, ADR 0008): comparisons with a float
// operand, the program captured on PUB400 as GN109FCM with DSPLY for its
// output.
// Source: golden capture tools/golden/native109 n109_floatcmp (PUB400, IBM
// i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s f8 float(8) inz(1.5);
dcl-s third float(8);
dcl-s p packed(5:2) inz(1.50);
dcl-s k int(5);
dcl-s line varchar(80);
if f8 > 1;
  line += 'a';
endif;
if f8 = p;
  line += 'b';
endif;
if f8 <> p;
  line += 'X';
endif;
third = 1 / 3;
if %float(1) / 3 = third;
  line += 'c';
endif;
if f8 >= 1.5 and f8 < 2;
  line += 'd';
endif;
dow f8 < 100;
  f8 = f8 * 3;
  k += 1;
enddo;
line += %char(k) + ' ' + local();
dsply (line);
*inlr = *on;

dcl-proc local;
  dcl-pi *n varchar(10);
  end-pi;
  dcl-s lf float(8) inz(-2);
  if lf < 0;
    return 'neg';
  endif;
  return 'pos';
end-proc;
