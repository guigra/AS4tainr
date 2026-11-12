**FREE
// A CPU-bound program for comparing engines (scripts/native-bench.sh):
// packed and zoned arithmetic, a character build, and a subroutine per
// iteration. Its output is the same on both engines; only the time is
// measured.
dcl-s i int(10);
dcl-s total packed(15:2) inz(0);
dcl-s z zoned(9:2);
dcl-s s varchar(64);
dcl-s count int(10) inz(0);
for i = 1 to 200000;
  z = i / 7;
  total += z * 1.05;
  exsr build;
endfor;
dsply ('TOTAL ' + %char(total));
dsply ('COUNT ' + %char(count));
*inlr = *on;

begsr build;
  s = %trim(%char(i)) + '-' + %subst('ABCDEFGHIJ' : %rem(i : 10) + 1 : 1);
  if %len(s) > 3;
    count += 1;
  endif;
endsr;
