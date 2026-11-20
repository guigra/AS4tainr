**FREE
// Stage 2, S2.160: comparisons of pointer expressions: pointer
// arithmetic and %ADDR on either side, <, >, = and <> *NULL. Captured on
// PUB400 (tools/golden/native160.txt n160_ptrcmp) for the native unit
// NPTRCMP.
ctl-opt dftactgrp(*no);
dcl-s buf char(20);
dcl-s p pointer;
dcl-s line varchar(80);

p = %addr(buf);
if %addr(buf) + 1 < %addr(buf) + 2;
  line = 'lt';
endif;
if p + 5 > p;
  line += ' gt';
endif;
if p + 0 = %addr(buf);
  line += ' eq';
endif;
if %addr(buf) + 3 = p + 4;
  line += ' bad';
endif;
if %addr(buf) <> *null;
  line += ' nn';
endif;
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
