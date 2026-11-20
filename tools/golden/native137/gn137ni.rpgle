**FREE
// Stage 2, S2.137: %NULLIND of a null-capable date field read from a file
// under ALWNULL(*USRCTL): tested, in %CHAR, set off, and after a record
// that is not null. GN137N has (1, NULL) and (2, 2026-01-31). Captured on
// PUB400 (tools/golden/native137.txt n137_nullind) for the native unit
// NNULLIND.
ctl-opt dftactgrp(*no) alwnull(*usrctl);
dcl-f gn137n usage(*input) rename(gn137n:rec137)
             extfile('GUIGRA261/GN137N');
dcl-s line varchar(80);

read gn137n;
line = %char(id) + %char(%nullind(d));
if %nullind(d);
  line += 'n';
endif;
%nullind(d) = *off;
line += %char(%nullind(d));
read gn137n;
line += ' ' + %char(id) + %char(%nullind(d)) + ' ' + %char(d);
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
