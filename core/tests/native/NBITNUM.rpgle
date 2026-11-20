**FREE
// Native unit NBITNUM (S2.153): bit built-in functions of numbers, the
// program captured on PUB400 as GN153BT with DSPLY for its output.
// Source: golden capture tools/golden/native153 n153_bitnum (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-c SRCBIT x'0800';
dcl-c LOWBIT x'04';
dcl-s flg uns(5) inz(2052);
dcl-s low uns(3) inz(6);
dcl-s n int(10) inz(12);
dcl-s line varchar(80);

if %bitand(flg : SRCBIT) = SRCBIT;
  line = 'src';
endif;
if %bitand(low : LOWBIT) = LOWBIT;
  line += ' low';
endif;
if %bitand(low : x'01') = x'01';
  line += ' odd';
endif;
line += ' ' + %char(%bitand(n : 10)) + ' ' + %char(%bitor(n : 3))
      + ' ' + %char(%bitxor(n : 5)) + ' ' + %char(%bitxor(n : x'10')) + ' ' + %char(%bitnot(n));
dsply (line);
*inlr = *on;
