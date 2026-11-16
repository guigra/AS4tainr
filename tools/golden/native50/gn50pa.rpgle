**FREE
// Stage 2, S2.49: pointer arithmetic: a pointer plus and less a number,
// a chain of both, a number field as the step, a pointer less a pointer,
// and *NULL plus a number (MONITOR). Captured on PUB400
// (tools/golden/native50.txt n50_pa) for the native unit NPARITH.
ctl-opt dftactgrp(*no);
dcl-s buf char(10) inz('ABCDEFGHIJ');
dcl-s p pointer;
dcl-s q pointer;
dcl-s c char(1) based(p);
dcl-s k int(10) inz(3);
dcl-s d int(10);
p = %addr(buf) + 1;
say('PLUS [' + c + ']');
p = p + k;
say('FIELD [' + c + ']');
p = p - 2;
say('LESS [' + c + ']');
p = %addr(buf) + 9 - 4 + 1;
say('CHAIN [' + c + ']');
q = %addr(buf);
d = p - q;
say('DIFF ' + %char(d));
p = *null;
monitor;
  p = p + 1;
  say('NULL MOVED');
on-error;
  say('NULL ERROR ' + %char(%status));
endmon;
*inlr = *on;
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
