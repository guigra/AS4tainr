**FREE
// Stage 2, S2.174: UPDATE with %FIELDS: a record read, two fields
// changed, only one of them named in %FIELDS, the record read again.
// GN174F holds the row 1 'aa' 'xx'. Captured on PUB400
// (tools/golden/native174.txt n174_updfld) for the native unit NUPDFLD.
ctl-opt dftactgrp(*no);
dcl-f gn174f usage(*update) rename(gn174f : gn174r);
dcl-s line varchar(80);

read gn174f;
s = 'zz';
t = 'yy';
update gn174r %fields(t);
setll *start gn174f;
read gn174f;
line = %char(n) + ' ' + s + ' ' + t;
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
