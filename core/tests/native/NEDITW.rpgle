**FREE
// Stage 2, S2.29: %EDITW, lowered to Rust. Expected: what IBM i 7.5 gave
// (golden capture tools/golden/native37.txt n37_editw, cited in
// native_units.rs). The capture's
// case EW DOLLAR is left out: the interpreter differs there
// (docs/native/fidelity.md).
ctl-opt dftactgrp(*no);
dcl-s p packed(9:2) inz(-1234.56);
dcl-s q packed(9:2) inz(1234.56);
dcl-s z packed(9:2) inz(0);
dcl-s i int(10) inz(1234567);
dcl-s r varchar(40);
r = %editw(p:'   ,   ,  0.  ');
dsply ('EW PLAIN [' + r + ']');
r = %editw(p:'   ,   ,  0.  CR');
dsply ('EW CR [' + r + ']');
r = %editw(p:'   ,   ,  0.  -');
dsply ('EW MINUS [' + r + ']');
r = %editw(p:'   ,   ,  *.  ');
dsply ('EW ASTER [' + r + ']');
r = %editw(p:'   &   &  0.  ');
dsply ('EW AMP [' + r + ']');
r = %editw(z:'   ,   ,  0.  ');
dsply ('EW ZERO [' + r + ']');
r = %editw(z:'   ,   ,  0.  CR');
dsply ('EW ZERO CR [' + r + ']');
r = %editw(i:'    ,   ,   ');
dsply ('EW INT [' + r + ']');
r = %editw(q:'   ,   ,  0.  CR');
dsply ('EW POS CR [' + r + ']');
r = %editw(p * 2:'    ,   ,  0.  ');
dsply ('EW EXPR [' + r + ']');
*inlr = *on;
