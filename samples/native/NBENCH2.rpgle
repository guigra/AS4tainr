**FREE
// A CPU-bound program for comparing engines (scripts/native-bench.sh,
// NATIVE_BENCH_PGM=NBENCH2), written with what stage 2 lowers to Rust:
// a FOR loop, packed arithmetic, array elements with literal indexes,
// qualified subfields, %SUBST, %SCAN, %TRIM, %CHAR and conditions. Its
// output is the same on both engines; only the time is measured.
dcl-s i int(10);
dcl-s tot packed(15:2) inz(0);
dcl-s amt packed(11:2);
dcl-s name char(10) inz('ACMEWIDGET');
dcl-s part char(3);
dcl-s hits int(10) inz(0);
dcl-s rate packed(5:3) dim(3);
dcl-ds rec qualified;
  qty packed(7:0);
  code char(4);
  label char(12);
end-ds;
rate(1) = 1.050;
rate(2) = 0.975;
rate(3) = 1.000;
for i = 1 to 200000;
  rec.qty = i;
  amt = rec.qty * rate(1);
  tot = tot + amt - rate(2) * rate(3);
  part = %subst(name : 3 : 3);
  if %scan('W' : name) > 0 and part = 'MEW';
    hits = hits + 1;
  endif;
  rec.code = %trim(part) + 'X';
  rec.label = rec.code + %char(rate(2));
endfor;
dsply ('TOT ' + %char(tot));
dsply ('HITS ' + %char(hits) + ' ' + rec.label);
*inlr = *on;
