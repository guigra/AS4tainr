**FREE
// Stage 2, S2.35: procedure calls within the module. BYREF changes seen by
// the caller, two parameters aliasing one variable, forwarding, VALUE
// copies, CONST by reference and by temporary, calls nested in arithmetic
// and return values, %PARMS, an error in the callee inside the caller's
// MONITOR, %STATUS after a normal return. Captured on PUB400
// (tools/golden/native44.txt n44_call) for the native unit test NCALL.
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
say('BYREF ' + %char(a));
twice(b : b);
say('ALIAS ' + %char(b));
fwd(a);
say('FWD ' + %char(a));
k = 5;
r = byval(k);
say('VALUE ' + %char(k) + ' ' + %char(r));
r = cref(a) + cref(a * 2);
say('CONST ' + %char(r));
r = add2(add2(1 : 2) : add2(3 : 4)) * 2;
say('NESTED ' + %char(r));
say('RETURN ' + %char(ret3()));
k = cnt(1);
say('PARMS 1 ' + %char(k));
k = cnt(1 : 2);
say('PARMS 2 ' + %char(k));
t = upc(c) + '!';
say('CHAR [' + t + ']');
monitor;
  r = divz(5);
  say('ERR NONE');
on-error;
  st(1) = %status;
endmon;
r = add2(1 : 1);
st(2) = %status;
say('ERR ' + %char(st(1)) + ' AFTER ' + %char(st(2)));
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

// Write one line to gn1.out in the job's current directory (DSPLY under
// `system` goes to QSYSOPR, a queue PUB400 users share).
dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s line varchar(200);
  line = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :line,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
