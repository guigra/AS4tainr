**FREE
// Stage 2, S2.35: procedure calls within the module, natively. Expected:
// what IBM i 7.5 gave (golden capture tools/golden/native44.txt n44_call,
// cited in native_units.rs).
ctl-opt dftactgrp(*no);
dcl-s a packed(7:2) inz(10.50);
dcl-s b packed(7:2) inz(3);
dcl-s r packed(9:2);
dcl-s k int(10);
dcl-s z packed(5:0) inz(0);
dcl-s t varchar(20);
dcl-s c char(5) inz('abc');
dcl-s st int(10) dim(2);

bump(a);
dsply ('BYREF ' + %char(a));
twice(b : b);
dsply ('ALIAS ' + %char(b));
fwd(a);
dsply ('FWD ' + %char(a));
k = 5;
r = byval(k);
dsply ('VALUE ' + %char(k) + ' ' + %char(r));
r = cref(a) + cref(a * 2);
dsply ('CONST ' + %char(r));
r = add2(add2(1 : 2) : add2(3 : 4)) * 2;
dsply ('NESTED ' + %char(r));
dsply ('RETURN ' + %char(ret3()));
k = cnt(1);
dsply ('PARMS 1 ' + %char(k));
k = cnt(1 : 2);
dsply ('PARMS 2 ' + %char(k));
t = upc(c) + '!';
dsply ('CHAR [' + t + ']');
monitor;
  r = divz(5);
  dsply ('ERR NONE');
on-error;
  st(1) = %status;
endmon;
r = add2(1 : 1);
st(2) = %status;
dsply ('ERR ' + %char(st(1)) + ' AFTER ' + %char(st(2)));
*inlr = *on;

dcl-proc bump;
  dcl-pi *n;
    x packed(7:2);
  end-pi;
  x += 1;
end-proc;

dcl-proc twice;
  dcl-pi *n;
    x packed(7:2);
    y packed(7:2);
  end-pi;
  x = x * 2;
  y = y + 1;
end-proc;

dcl-proc fwd;
  dcl-pi *n;
    x packed(7:2);
  end-pi;
  bump(x);
  x = x * 10;
end-proc;

dcl-proc byval;
  dcl-pi *n packed(9:2);
    x int(10) value;
  end-pi;
  x = x * 3;
  return x + 0.25;
end-proc;

dcl-proc cref;
  dcl-pi *n packed(9:2);
    x packed(7:2) const;
  end-pi;
  return x + 1;
end-proc;

dcl-proc add2;
  dcl-pi *n packed(9:2);
    x packed(9:2) value;
    y packed(9:2) value;
  end-pi;
  return x + y;
end-proc;

dcl-proc ret3;
  dcl-pi *n packed(3:1);
  end-pi;
  return 12.345;
end-proc;

dcl-proc cnt;
  dcl-pi *n int(10);
    x int(10) value;
    y int(10) value options(*nopass);
  end-pi;
  return %parms;
end-proc;

dcl-proc upc;
  dcl-pi *n varchar(10);
    x char(5) const;
  end-pi;
  return %trimr(x) + 'X';
end-proc;

dcl-proc divz;
  dcl-pi *n packed(9:2);
    x packed(5:0) value;
  end-pi;
  return x / z;
end-proc;
