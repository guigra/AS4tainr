**FREE
// Stage 2, S2.8: lowered code inside procedures: locals, a FOR loop and
// EVAL on them, a parameter passed by reference changed, a CONST
// parameter read, recursion. Expected values are the arithmetic (1+...+5
// = 15; 15 + 2.5 = 17.5; 6! = 720) under the parameter rules of IBM Docs
// "Passing by Reference" and "Passing by Value" (cited in
// native_units.rs).
ctl-opt dftactgrp(*no);
dcl-s total packed(9:2) inz(0);
dcl-s n int(10) inz(5);
total = sumto(n);
if total = 15;
  dsply 'PROC LOOP OK';
else;
  dsply 'PROC LOOP FAIL';
endif;
addinto(total : 2.5);
if total = 17.5;
  dsply 'PROC BYREF OK';
else;
  dsply 'PROC BYREF FAIL';
endif;
if fact(6) = 720;
  dsply 'PROC RECURSION OK';
else;
  dsply 'PROC RECURSION FAIL';
endif;
*inlr = *on;

dcl-proc sumto;
  dcl-pi *n packed(9:2);
    lim int(10) value;
  end-pi;
  dcl-s i int(10);
  dcl-s acc packed(9:2) inz(0);
  for i = 1 to lim;
    acc += i;
  endfor;
  return acc;
end-proc;

dcl-proc addinto;
  dcl-pi *n;
    t packed(9:2);
    amt packed(5:1) const;
  end-pi;
  t = t + amt;
end-proc;

dcl-proc fact;
  dcl-pi *n int(20);
    k int(10) value;
  end-pi;
  dcl-s r int(20);
  if k <= 1;
    return 1;
  endif;
  r = k * fact(k - 1);
  return r;
end-proc;
