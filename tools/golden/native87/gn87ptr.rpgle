**FREE
// Stage 2, S2.87: procedure pointers. %PADDR of a procedure by name and
// by literal, calls through a prototype with EXTPROC(pointer) as the
// pointer is changed, the pointer passed and compared, and a call through
// a pointer set to *NULL caught by MONITOR (its status). Captured on
// PUB400 (tools/golden/native87.txt n87_procptr) for the native unit
// NPROCPTR.
ctl-opt dftactgrp(*no);
dcl-s pp pointer(*proc);
dcl-s qq pointer(*proc);
dcl-pr op int(10) extproc(pp);
  a int(10) value;
  b int(10) value;
end-pr;
dcl-s line varchar(52);
pp = %paddr(add);
line = 'ADD ' + %char(op(2: 3));
pp = %paddr('MUL');
line += ' MUL ' + %char(op(2: 3));
qq = %paddr(add);
if pp <> qq;
  line += ' NE';
endif;
pp = qq;
if pp = qq;
  line += ' EQ ' + %char(op(10: 5));
endif;
say(line);
pp = *null;
monitor;
  line = 'CALL ' + %char(op(1: 1));
on-error;
  line = 'NULL ' + %char(%status);
endmon;
say(line);
*inlr = *on;

dcl-proc add;
  dcl-pi *n int(10);
    a int(10) value;
    b int(10) value;
  end-pi;
  return a + b;
end-proc;

dcl-proc mul;
  dcl-pi *n int(10);
    a int(10) value;
    b int(10) value;
  end-pi;
  return a * b;
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
