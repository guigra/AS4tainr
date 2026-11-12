**FREE
// Stage 2, S2.4: character EVAL, EVALR, concatenation and comparisons,
// lowered to Rust. Expected: what IBM i 7.5 gave (golden capture
// tools/golden/native4.txt n4_char, cited in native_units.rs).
// Character fields: EVAL and EVALR (padding, truncation), concatenation
// of fixed-length fields (their trailing blanks kept), and comparisons of
// different lengths.
ctl-opt dftactgrp(*no);
dcl-s a char(5) inz('AB');
dcl-s b char(3) inz('XYZ');
dcl-s c char(10);
dcl-s d char(4);
dcl-s e char(7) inz('AB');
c = a + b;
dsply ('CHR CAT [' + c + ']');
d = a + b;
dsply ('CHR TRUNC [' + d + ']');
evalr c = b;
dsply ('CHR EVALR [' + c + ']');
evalr d = a + b;
dsply ('CHR EVALR TRUNC [' + d + ']');
c = 'Q';
dsply ('CHR PAD [' + c + ']');
if a = e;
  dsply ('CHR CMP PADDED EQ');
else;
  dsply ('CHR CMP PADDED NE');
endif;
if a < b;
  dsply ('CHR CMP LT');
else;
  dsply ('CHR CMP GE');
endif;
if 'ab' < 'AB';
  dsply ('CHR CMP LOWER FIRST');
else;
  dsply ('CHR CMP UPPER FIRST');
endif;
if '1' > 'Z';
  dsply ('CHR CMP DIGIT HIGH');
else;
  dsply ('CHR CMP DIGIT LOW');
endif;
*inlr = *on;
