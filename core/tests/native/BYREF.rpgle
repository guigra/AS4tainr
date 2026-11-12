**FREE
// By reference is ILE RPG's default style: the callee works on the
// caller's field (docs cited in core/tests/native_units.rs).
ctl-opt dftactgrp(*no);
dcl-s x int(10) inz(5);
dcl-s y int(10) inz(1);
setTen(x);
if x = 10;
  dsply 'BYREF OK';
else;
  dsply 'BYREF FAIL';
endif;
// Forwarded: A passes its own reference parameter on to B.
passOn(y);
if y = 7;
  dsply 'FORWARD OK';
else;
  dsply 'FORWARD FAIL';
endif;
*inlr = *on;

dcl-proc setTen;
  dcl-pi *n;
    p int(10);
  end-pi;
  p = 10;
end-proc;

dcl-proc passOn;
  dcl-pi *n;
    p int(10);
  end-pi;
  setSeven(p);
end-proc;

dcl-proc setSeven;
  dcl-pi *n;
    p int(10);
  end-pi;
  p = 7;
end-proc;
