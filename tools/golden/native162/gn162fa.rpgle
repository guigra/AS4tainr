**FREE
// Stage 2, S2.162: %ABS of float expressions into float fields: a field,
// an expression, and one too large for a decimal of 63 digits. Captured
// on PUB400 (tools/golden/native162.txt n162_fltabs) for the native unit
// NFLTABS.
ctl-opt dftactgrp(*no);
dcl-s f float(8);
dcl-s d float(8);
dcl-s g float(4);
dcl-s line varchar(120);

f = -2.5E0;
d = %abs(f);
line = %char(d);
d = %abs(f * 3 - 1);
line += ' ' + %char(d);
g = %abs(f / 4);
line += ' ' + %char(g);
f = -1.797693134862315E+308;
d = %abs(f - f / 2);
if f < 0 and d > 0;
  line += ' huge';
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
