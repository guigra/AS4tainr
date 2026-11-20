**FREE
// Stage 2, S2.158: %STR as an assignment's target, with a literal and a
// %SIZE maximum, through a pointer and a pointer moved on, the null it
// writes and the bytes after it left. Captured on PUB400
// (tools/golden/native158.txt n158_strset) for the native unit NSTRSET.
ctl-opt dftactgrp(*no);
dcl-s buf char(12) inz(*all'x');
dcl-s small char(4);
dcl-s p pointer;
dcl-s line varchar(80);

p = %addr(buf);
%str(p : 6) = 'Hello world';
line = %str(p) + ' ' + %char(%len(%str(p))) + ' ' + %subst(buf : 7 : 6);
%str(p + 2 : 4) = 'ab';
line += ' ' + %str(p);
%str(%addr(small) : %size(small)) = 'zzzzzz';
line += ' ' + %str(%addr(small));
say(line);
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
