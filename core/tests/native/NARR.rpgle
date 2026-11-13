**FREE
// Stage 2, S2.15: array elements with literal indexes, lowered to Rust.
// Expected: what IBM i 7.5 gave (golden capture tools/golden/native16.txt
// n16_arr, cited in native_units.rs).
ctl-opt dftactgrp(*no);
dcl-s arr packed(5:2) dim(5);
dcl-s names char(4) dim(3);
dcl-s tot packed(7:2);
dcl-s i int(10);
dcl-ds d;
  sub char(2) dim(3);
end-ds;
arr(1) = 1.25;
arr(5) = arr(1) * 2;
dsply ('ARR FIRST LAST ' + %char(arr(1)) + ' ' + %char(arr(5)));
names(2) = 'AB';
names(3) = %trim(names(2)) + 'C';
dsply ('ARR CHAR [' + names(1) + '|' + names(2) + '|' + names(3) + ']');
if arr(5) > arr(1);
  dsply ('ARR COND GT');
else;
  dsply ('ARR COND LE');
endif;
arr(3) = arr(5) - arr(1);
for i = 1 to 5;
  tot = tot + arr(i);
endfor;
dsply ('ARR SUM ' + %char(tot));
sub(2) = 'XY';
sub(1) = sub(2);
dsply ('ARR DS [' + d + ']');
*inlr = *on;
