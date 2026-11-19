**FREE
// Stage 2, S2.125: a whole data structure passed to character parameters
// of local procedures: by reference, with OPTIONS(*VARSIZE), CONST of
// the same and of another length, and VALUE. Captured on PUB400
// (tools/golden/native125.txt n125_dsparm) for the native unit NDSCHRP.
ctl-opt dftactgrp(*no);
dcl-ds d qualified;
  a char(3) inz('abc');
  n zoned(3:0) inz(42);
end-ds;
dcl-s line varchar(80);

byref(d);
line = d.a + %char(d.n);
vsize(d);
line += ' ' + d.a;
line += ' ' + cst(d);
line += ' ' + %char(%len(%trimr(cstlong(d))));
val(d);
line += ' ' + d.a;
say(line);
*inlr = *on;

dcl-proc byref;
  dcl-pi *n;
    x char(6);
  end-pi;
  x = 'XYZ999';
end-proc;

dcl-proc vsize;
  dcl-pi *n;
    x char(100) options(*varsize);
  end-pi;
  %subst(x: 1: 1) = 'Q';
end-proc;

dcl-proc cst;
  dcl-pi *n char(6);
    x char(6) const;
  end-pi;
  return x;
end-proc;

dcl-proc cstlong;
  dcl-pi *n char(10);
    x char(10) const;
  end-pi;
  return x;
end-proc;

dcl-proc val;
  dcl-pi *n;
    x char(6) value;
  end-pi;
  x = 'ZZZZZZ';
end-proc;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
