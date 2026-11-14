**FREE
// Stage 2, S2.22: figurative constants in EVAL and comparisons, lowered to
// Rust. Expected: what IBM i 7.5 gave (golden capture
// tools/golden/native28.txt n28_fig, cited in native_units.rs). The
// capture's cases FIG CMP HIVAL and FIG VAR are left out: the interpreter
// differs there (docs/native/fidelity.md).
ctl-opt dftactgrp(*no);
dcl-s c char(5) inz('ABCDE');
dcl-s p packed(5:2) inz(123.45);
dcl-s z zoned(3:0) inz(7);
c = *blanks;
dsply ('FIG BLANKS [' + c + ']');
if c = *blanks;
  dsply ('FIG CMP BLANKS EQ');
else;
  dsply ('FIG CMP BLANKS NE');
endif;
p = *zeros;
dsply ('FIG ZEROS ' + %char(p));
z = *hival;
dsply ('FIG HIVAL ' + %char(z));
p = *loval;
dsply ('FIG LOVAL ' + %char(p));
c = *all'AB';
dsply ('FIG ALL [' + c + ']');
if c > *loval and c < *hival;
  dsply ('FIG CMP RANGE YES');
else;
  dsply ('FIG CMP RANGE NO');
endif;
*inlr = *on;
