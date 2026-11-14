**FREE
// Stage 2, S2.24: a whole data structure as a character operand, lowered
// to Rust. Expected: what IBM i 7.5 gave (golden capture
// tools/golden/native31.txt n31_dsw, cited in native_units.rs).
ctl-opt dftactgrp(*no);
dcl-ds rec;
  code char(3);
  qty zoned(3:0);
  name char(4);
end-ds;
dcl-s c char(12);
rec = 'ABC012WXYZ';
dsply ('DSW SUBS [' + code + '|' + %char(qty) + '|' + name + ']');
c = rec;
dsply ('DSW TO CHAR [' + c + ']');
rec = 'XY';
dsply ('DSW PAD [' + %subst(rec:1:3) + ']');
if rec = 'XY';
  dsply ('DSW CMP PADDED EQ');
else;
  dsply ('DSW CMP PADDED NE');
endif;
qty = 7;
c = rec;
dsply ('DSW AFTER SUB [' + c + ']');
evalr rec = 'Q';
dsply ('DSW EVALR [' + name + ']');
dsply ('DSW LEN ' + %char(%len(rec)));
*inlr = *on;
