**FREE
// Calls inside arithmetic and in return values, and recursion.
ctl-opt dftactgrp(*no);
dcl-s r int(10);
r = double(3) + double(4);
if r = 14;
  dsply 'ARITH OK';
else;
  dsply 'ARITH FAIL';
endif;
r = quad(2);
if r = 8;
  dsply 'NESTED OK';
else;
  dsply 'NESTED FAIL';
endif;
if fact(10) = 3628800;
  dsply 'RECURSION OK';
else;
  dsply 'RECURSION FAIL';
endif;
*inlr = *on;

dcl-proc double;
  dcl-pi *n int(10);
    n int(10) value;
  end-pi;
  return n * 2;
end-proc;

dcl-proc quad;
  dcl-pi *n int(10);
    n int(10) value;
  end-pi;
  return double(double(n));
end-proc;

dcl-proc fact;
  dcl-pi *n int(20);
    n int(10) value;
  end-pi;
  if n <= 1;
    return 1;
  endif;
  return n * fact(n - 1);
end-proc;
