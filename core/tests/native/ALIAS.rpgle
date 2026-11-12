**FREE
// Two reference parameters that are one variable share its storage: on
// IBM i, after p1 = 1 and p2 = 2 both are 2, and so is the caller's
// variable (golden capture tools/golden/native1.txt n1_alias, cited in
// native_units.rs).
ctl-opt dftactgrp(*no);
dcl-s v int(10) inz(0);
dcl-s seen1 int(10);
dcl-s seen2 int(10);
both(v : v);
if seen1 = 2 and seen2 = 2 and v = 2;
  dsply 'ALIAS OK';
else;
  dsply 'ALIAS FAIL';
endif;
*inlr = *on;
dcl-proc both;
  dcl-pi *n;
    p1 int(10);
    p2 int(10);
  end-pi;
  p1 = 1;
  p2 = 2;
  seen1 = p1;
  seen2 = p2;
end-proc;
