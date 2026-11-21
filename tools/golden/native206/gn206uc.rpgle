**FREE
// Stage 2, S2.206: UCS-2 fields with character data: a fixed and a
// varying UCS-2 field assigned character literals, %TRIM and %CHAR of
// them, concatenation with character data, and a comparison. Captured on
// PUB400 (tools/golden/native206.txt n206_ucs2) for the native unit
// NUCS2.
ctl-opt dftactgrp(*no);
dcl-s u ucs2(6);
dcl-s v varucs2(10);
dcl-s line varchar(200);

u = 'abc';
v = 'Bern';
line = '[' + %char(u) + '][' + %trim(v) + '][' + %char(v) + ']';
v = 'Zurich';
line += ' ' + %trim(v);
if u = 'abc';
  line += ' EQ';
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
