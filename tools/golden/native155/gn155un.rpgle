**FREE
// Stage 2, S2.155: %UNS and %UNSH of character and numeric values, a
// negative value and a value too large for the target in MONITOR groups.
// Captured on PUB400 (tools/golden/native155.txt n155_uns) for the
// native unit NUNSBIF.
ctl-opt dftactgrp(*no);
dcl-s u5 uns(5);
dcl-s u3 uns(3);
dcl-s p52 packed(5:2) inz(9.5);
dcl-s c char(6) inz(' 7.6  ');
dcl-s line varchar(80);

u5 = %uns(' 42 ');
line = %char(u5);
u5 = %unsh(c);
line += ' ' + %char(u5) + ' ' + %char(%uns(p52)) + ' ' + %char(%unsh(p52));
line += ' ' + %char(%uns('12.99') + 1);
monitor;
  u5 = %uns('-3');
  line += ' no error';
on-error;
  line += ' ' + %char(%status);
endmon;
monitor;
  u3 = %uns('300');
  line += ' no error';
on-error;
  line += ' ' + %char(%status);
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
