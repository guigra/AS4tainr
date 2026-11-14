**FREE
// Stage 2, S2.21: array elements with an index that is a field, lowered to
// Rust. Expected: what IBM i 7.5 gave (golden capture
// tools/golden/native27.txt n27_arrv, cited in native_units.rs).
ctl-opt dftactgrp(*no);
dcl-s arr packed(5:0) dim(5);
dcl-s names char(3) dim(4);
dcl-s i int(10);
dcl-s j int(10);
dcl-s tot packed(7:0);
for i = 1 to 5;
  arr(i) = i * 10;
endfor;
for i = 1 to 5;
  tot = tot + arr(i);
endfor;
dsply ('ARRV SUM ' + %char(tot));
i = 3;
names(i) = 'ABC';
j = 2;
names(j) = names(i);
dsply ('ARRV CHAR [' + names(1) + '|' + names(2) + '|' + names(3) + ']');
if arr(i) > 20;
  dsply ('ARRV COND GT');
else;
  dsply ('ARRV COND LE');
endif;
arr(j) = arr(i) + arr(j);
dsply ('ARRV EXPR ' + %char(arr(2)));
i = 6;
monitor;
  tot = arr(i);
  dsply ('ARRV PAST ' + %char(tot));
on-error;
  dsply ('ARRV PAST ERR ' + %char(%status));
endmon;
i = 0;
monitor;
  arr(i) = 1;
  dsply ('ARRV ZERO OK');
on-error;
  dsply ('ARRV ZERO ERR ' + %char(%status));
endmon;
*inlr = *on;
