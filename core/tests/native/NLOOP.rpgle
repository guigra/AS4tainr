**FREE
// Stage 2 (docs/native/adr/0006): FOR/DO control and conditions on
// global numeric fields, lowered to Rust. Expected values follow the
// procedure of IBM Docs "FOR (For)" (cited in native_units.rs): the
// index is compared with the limit before each pass and stepped after
// it, so it ends one step past the limit.
dcl-s i packed(5:0);
dcl-s n int(10) inz(0);
dcl-s z zoned(7:2) inz(0);
dcl-s small packed(3:0);
for i = 1 to 10 by 3;
  n += 1;
endfor;
if n = 4 and i = 13;
  dsply 'FOR TO OK';
else;
  dsply 'FOR TO FAIL';
endif;
for i = 10 downto 1 by 4;
  n += 1;
endfor;
if n = 7 and i = -2;
  dsply 'FOR DOWNTO OK';
else;
  dsply 'FOR DOWNTO FAIL';
endif;
// A loop that does not run: the limit is checked first.
for i = 5 to 4;
  n += 100;
endfor;
if n = 7 and i = 5;
  dsply 'FOR ZERO OK';
else;
  dsply 'FOR ZERO FAIL';
endif;
dow n < 10 or z <> 0;
  n += 1;
enddo;
if n = 10 and (z = 0 or n > 99);
  dsply 'DOW OK';
else;
  dsply 'DOW FAIL';
endif;
// The index must hold one more than the limit: 1000 does not fit a
// packed(3:0), so the step is an error (status 00103, a value too large
// for its field).
monitor;
  for small = 995 to 999;
  endfor;
  dsply 'INDEX OVERFLOW FAIL';
on-error 00103;
  dsply 'INDEX OVERFLOW OK';
endmon;
*inlr = *on;
