**FREE
// Stage 2, S2.124: a based data structure as a whole: passed by
// reference with OPTIONS(*VARSIZE), taken by %SUBST and assigned.
// Captured on PUB400 (tools/golden/native124.txt n124_basedds) for the
// native unit NBASEDDS.
ctl-opt dftactgrp(*no);
dcl-ds hdr qualified based(p);
  a int(10);
  b char(4);
end-ds;
dcl-s p pointer;
dcl-s line varchar(80);
dcl-pr fill;
  buf char(100) options(*varsize);
  n int(10) const;
end-pr;

p = %alloc(8);
hdr.a = 7;
hdr.b = 'abcd';
line = %char(hdr.a) + hdr.b;
fill(hdr: %size(hdr));
line += ' ' + %subst(hdr: 5: 4);
hdr = 'ABCDEFGH';
line += ' ' + hdr.b;
dealloc p;
say(line);
*inlr = *on;

dcl-proc fill;
  dcl-pi *n;
    buf char(100) options(*varsize);
    n int(10) const;
  end-pi;
  %subst(buf: 5: 4) = 'wxyz';
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
