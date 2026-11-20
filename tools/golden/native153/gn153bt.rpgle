**FREE
// Stage 2, S2.153: %BITAND, %BITOR, %BITXOR and %BITNOT of numbers, with
// hexadecimal constants and literals as unsigned values, in a comparison
// and in %CHAR. Captured on PUB400 (tools/golden/native153.txt
// n153_bitnum) for the native unit NBITNUM.
ctl-opt dftactgrp(*no);
dcl-c SRCBIT x'0800';
dcl-c LOWBIT x'04';
dcl-s flg uns(5) inz(2052);
dcl-s low uns(3) inz(6);
dcl-s n int(10) inz(12);
dcl-s line varchar(80);

if %bitand(flg : SRCBIT) = SRCBIT;
  line = 'src';
endif;
if %bitand(low : LOWBIT) = LOWBIT;
  line += ' low';
endif;
if %bitand(low : x'01') = x'01';
  line += ' odd';
endif;
line += ' ' + %char(%bitand(n : 10)) + ' ' + %char(%bitor(n : 3))
      + ' ' + %char(%bitxor(n : 5)) + ' ' + %char(%bitxor(n : x'10')) + ' ' + %char(%bitnot(n));
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
