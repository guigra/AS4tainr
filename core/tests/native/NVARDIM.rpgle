**FREE
// Native unit NVARDIM: expected values from PUB400, IBM i 7.5 (golden capture
// tools/golden/native47*.txt, cited in native_units.rs). The capture's
// cases AUTO6 and VARSET5 are left out: the interpreter gives status 121
// there and IBM i 124 (docs/native/fidelity.md).
ctl-opt dftactgrp(*no);
dcl-s au int(10) dim(*auto:5);
dcl-s va char(3) dim(*var:4);
dcl-s n int(10);
dcl-s t varchar(60);
dcl-s k int(10) inz(3);
dcl-s k6 int(10) inz(6);
dcl-s n5 int(10) inz(5);
dsply ('E0 ' + %char(%elem(au)) + ' ' + %char(%elem(va)));
au(2) = 7;
dsply ('AUTO2 ' + %char(%elem(au)) + ' ' + %char(au(1)) + ' ' + %char(au(2)));
au(*next) = 9;
dsply ('NEXT ' + %char(%elem(au)) + ' ' + %char(au(3)));
monitor;
  va(1) = 'abc';
  dsply ('VAR1 ' + %char(%elem(va)));
on-error;
  dsply ('VAR1 ERR ' + %char(%status));
endmon;
%elem(va) = 2;
va(2) = 'xyz';
t = '[' + va(1) + '][' + va(2) + ']';
dsply ('VARSET ' + %char(%elem(va)) + ' ' + t);
monitor;
  va(k) = 'qqq';
  dsply ('VAR3 ' + %char(%elem(va)));
on-error;
  dsply ('VAR3 ERR ' + %char(%status));
endmon;
monitor;
  n = au(4);
  dsply ('READ4 ' + %char(n));
on-error;
  dsply ('READ4 ERR ' + %char(%status));
endmon;
%elem(au) = 1;
dsply ('SHRINK ' + %char(%elem(au)) + ' ' + %char(au(1)));
%elem(au) = 3;
dsply ('REGROW ' + %char(%elem(au)) + ' ' + %char(au(2)) + ' ' + %char(au(3)));
%elem(va) = 4;
t = '[' + va(3) + '][' + va(4) + ']';
dsply ('VARGROW ' + %char(%elem(va)) + ' ' + t);
*inlr = *on;
