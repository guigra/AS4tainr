**FREE
// Stage 2, S2.145: arithmetic in conditions: products, quotients (one to
// 28 decimal places, the precision rules' M = 30), sums, IN %LIST of a
// product, and a DOW on a difference. Captured on PUB400
// (tools/golden/native145.txt n145_cmparith) for the native unit NCMPAR.
ctl-opt dftactgrp(*no);
dcl-s a packed(5:2) inz(1.25);
dcl-s b packed(5:2) inz(1.20);
dcl-s n int(10) inz(7);
dcl-s line varchar(80);

if a * b = 1.5;
  line += 'a';
endif;
if n / 3 > 2.33;
  line += 'b';
endif;
if 10 / 3 = 3.3333333333333333333333333333;
  line += 'c';
endif;
if a + b - 2.45 = 0;
  line += 'd';
endif;
if n * 2 in %list(3 : 14);
  line += 'e';
endif;
if (a - b) * 10 < 0.6;
  line += 'f';
endif;
dow n - 5 > 0;
  n -= 1;
enddo;
line += %char(n);
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
