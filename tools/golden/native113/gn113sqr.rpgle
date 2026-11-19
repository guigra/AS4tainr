**FREE
// Stage 2, S2.113: %SQRT of 16, of 2 into packed(15:13), of a packed
// field with decimals, %CHAR of %SQRT(10), and %SQRT of a negative
// number under MONITOR. Captured on PUB400 (tools/golden/native113.txt
// n113_sqrt) for the native unit NSQRT.
ctl-opt dftactgrp(*no);
dcl-s p packed(15:13);
dcl-s q packed(7:2) inz(6.25);
dcl-s neg packed(3:0) inz(-1);
dcl-s line varchar(200);
p = %sqrt(16);
line = %char(p);
p = %sqrt(2);
line += ' ' + %char(p);
p = %sqrt(q);
line += ' ' + %char(p) + ' ' + %char(%sqrt(10));
monitor;
  p = %sqrt(neg);
  line += ' D';
on-error;
  line += ' E' + %char(%status);
endmon;
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
