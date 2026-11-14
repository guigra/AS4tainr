**FREE
// Stage 2, S2.30: subfields of a data structure array, lowered to Rust.
// Expected: what IBM i 7.5 gave (golden capture tools/golden/native38.txt
// n38_dsa, cited in native_units.rs).
ctl-opt dftactgrp(*no);
dcl-ds item qualified dim(3);
  code char(2);
  qty packed(5:0);
end-ds;
dcl-s i int(10);
dcl-s j int(10) inz(3);
dcl-s k int(10) inz(4);
dcl-s tot packed(7:0);
for i = 1 to 3;
  item(i).code = 'C' + %char(i);
  item(i).qty = i * 10;
endfor;
dsply ('DSA LIT [' + item(2).code + '] ' + %char(item(2).qty));
for i = 1 to 3;
  tot = tot + item(i).qty;
endfor;
dsply ('DSA SUM ' + %char(tot));
if item(j).qty > 20 and item(j).code = 'C3';
  dsply ('DSA COND YES');
else;
  dsply ('DSA COND NO');
endif;
item(1).code = item(j).code;
dsply ('DSA COPY [' + item(1).code + ']');
monitor;
  tot = item(k).qty;
  dsply ('DSA PAST ' + %char(tot));
on-error;
  dsply ('DSA PAST ERR ' + %char(%status));
endmon;
*inlr = *on;
