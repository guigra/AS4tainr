**FREE
// VALUE: the procedure works on its own copy; CONST: read-only, and an
// expression is passed through a temporary.
ctl-opt dftactgrp(*no);
dcl-s x int(10) inz(5);
dcl-s r int(10);
change(x);
if x = 5;
  dsply 'VALUE OK';
else;
  dsply 'VALUE FAIL';
endif;
r = twice(5 + 1);
if r = 12;
  dsply 'CONST OK';
else;
  dsply 'CONST FAIL';
endif;
*inlr = *on;

dcl-proc change;
  dcl-pi *n;
    p int(10) value;
  end-pi;
  p = 10;
end-proc;

dcl-proc twice;
  dcl-pi *n int(10);
    p int(10) const;
  end-pi;
  return p * 2;
end-proc;
