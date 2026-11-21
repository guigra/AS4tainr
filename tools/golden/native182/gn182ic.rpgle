**FREE
// Stage 2, S2.182: x IN an enumeration (character and numeric constants)
// and %CONCAT with a literal separator, *NONE and *BLANK, and a
// character field. Captured on PUB400 (tools/golden/native182.txt
// n182_inconcat) for the native unit NINCONC.
ctl-opt dftactgrp(*no);
dcl-enum color qualified;
  red 'R';
  green 'G';
  blue 'B';
end-enum;
dcl-enum size;
  small 1;
  big 9;
end-enum;
dcl-s c char(1) inz('G');
dcl-s n packed(3:0) inz(9);
dcl-s w char(4) inz('ab');
dcl-s line varchar(200);

if c in color;
  line = 'G-in ';
else;
  line = 'G-out ';
endif;
c = 'X';
if c in color;
  line += 'X-in ';
else;
  line += 'X-out ';
endif;
if n in size;
  line += '9-in|';
else;
  line += '9-out|';
endif;
line += %concat(', ' : 'x' : 'y' : 'z') + '|';
line += %concat(*none : 'a' : 'b') + '|';
line += %concat(*blank : 'a' : 'b') + '|';
line += %concat('-' : w : 'q' : w) + '|';
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
