**FREE
// Stage 2, S2.2 (docs/native/adr/0006): numeric EVAL of +, - and * on
// global fields, lowered to Rust. The expected values are what IBM i 7.5
// gave for the same statements (golden capture tools/golden/native2.txt
// n2_eval, cited in native_units.rs).
dcl-s x packed(5:2) inz(1.27);
dcl-s y packed(5:1) inz(2.5);
dcl-s r1 packed(5:1);
dcl-s r4 packed(9:4);
dcl-s n packed(5:2) inz(-1.27);
dcl-s s packed(7:3);
dcl-s small packed(3:0) inz(999);
r1 = x * y;
if r1 = 3.1;
  dsply 'EVAL TRUNC OK';
else;
  dsply 'EVAL TRUNC FAIL';
endif;
eval(h) r1 = x * y;
if r1 = 3.2;
  dsply 'EVAL HALF OK';
else;
  dsply 'EVAL HALF FAIL';
endif;
eval(h) r1 = n * y;
if r1 = -3.2;
  dsply 'EVAL HALF NEG OK';
else;
  dsply 'EVAL HALF NEG FAIL';
endif;
r4 = x * y;
if r4 = 3.1750;
  dsply 'EVAL FULL OK';
else;
  dsply 'EVAL FULL FAIL';
endif;
s = x + y - 0.001;
if s = 3.769;
  dsply 'EVAL ADDSUB OK';
else;
  dsply 'EVAL ADDSUB FAIL';
endif;
eval(h) r1 = 0.05 * 1;
if r1 = 0.1;
  dsply 'EVAL EDGE OK';
else;
  dsply 'EVAL EDGE FAIL';
endif;
// A result too large for its field: status 00103.
monitor;
  small = small + 1;
  dsply 'EVAL OVERFLOW FAIL';
on-error 00103;
  dsply 'EVAL OVERFLOW OK';
endmon;
*inlr = *on;
