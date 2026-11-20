**FREE
// Stage 2, S2.138: TEST(D), TEST(T), TEST(Z) with E, of character and
// numeric fields, valid and not, in *ISO and *USA. Captured on PUB400
// (tools/golden/native138.txt n138_test) for the native unit NTESTDT.
ctl-opt dftactgrp(*no);
dcl-s c10 char(10) inz('2026-02-30');
dcl-s n8 zoned(8:0) inz(20260131);
dcl-s tm char(8) inz('25.00.00');
dcl-s tz char(26) inz('2026-01-31-23.59.59.000000');
dcl-s line varchar(80);

test(de) *iso c10;
line = %char(%error);
test(de) *iso n8;
line += %char(%error);
test(te) *iso tm;
line += %char(%error);
test(ze) tz;
line += %char(%error);
c10 = '01/31/2026';
test(de) *usa c10;
line += %char(%error);
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
