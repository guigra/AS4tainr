**FREE
// Stage 2, S2.16: qualified subfields (QUALIFIED and LIKEDS), lowered to
// Rust. Expected: what IBM i 7.5 gave (golden capture
// tools/golden/native17.txt n17_dsq, cited in native_units.rs).
ctl-opt dftactgrp(*no);
dcl-ds cust qualified;
  id packed(5:0);
  name char(10);
end-ds;
dcl-ds tmpl qualified template;
  a char(3);
  n zoned(3:0);
end-ds;
dcl-ds rec likeds(tmpl);
cust.id = 42;
cust.name = 'ACME';
cust.id = cust.id + 1;
dsply ('DSQ CUST ' + %char(cust.id) + ' [' + cust.name + ']');
rec.n = cust.id * 2;
rec.a = %trim(cust.name);
dsply ('DSQ LIKEDS ' + %char(rec.n) + ' [' + rec.a + ']');
if rec.n > 80 and %subst(cust.name:1:2) = 'AC';
  dsply ('DSQ COND YES');
else;
  dsply ('DSQ COND NO');
endif;
dsply ('DSQ BYTES [' + %subst(rec:1:3) + ']');
*inlr = *on;
