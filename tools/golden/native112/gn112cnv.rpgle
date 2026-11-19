**FREE
// Stage 2, S2.112: %INT, %INTH and %DEC of character data (literals and
// a field), %INT of text that is no number under MONITOR, %ERROR after
// TEST(DE) of a bad date and after a second OPEN(E) of a printer file,
// and %STATUS of that file. Captured on PUB400 (tools/golden/native112.txt
// n112_charconv) for the native unit NCHRCNV.
ctl-opt dftactgrp(*no);
dcl-f qprint printer(132) usropn;
dcl-s txt char(8) inz(' 42 ');
dcl-s bad char(10) inz('2026-13-40');
dcl-s n int(10);
dcl-s p packed(5:2);
dcl-s line varchar(200);
n = %int('123');
line = %char(n);
n = %inth('12.5');
line += ' ' + %char(n);
p = %dec('-3.25' : 5 : 2);
line += ' ' + %char(p);
n = %int(txt);
line += ' ' + %char(n);
monitor;
  n = %int('abc');
  line += ' D';
on-error;
  line += ' E' + %char(%status);
endmon;
test(de) *iso bad;
if %error;
  line += ' err';
endif;
line += ' ' + %char(%error);
open qprint;
open(e) qprint;
line += ' ' + %char(%error) + ' ' + %char(%status(qprint));
close qprint;
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
