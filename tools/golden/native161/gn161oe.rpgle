**FREE
// Stage 2, S2.161: OPEN(E), CLOSE(E) and FEOD(E) of a printer file, and
// CLOSE(E) of a database file already closed: %ERROR and %STATUS after
// each. Captured on PUB400 (tools/golden/native161.txt n161_opene) for
// the native unit NOPENE. GN120F is native120's table.
ctl-opt dftactgrp(*no);
dcl-f gn120f usage(*input) usropn rename(gn120f : gn120r);
dcl-f qprint printer(132) usropn;
dcl-s line varchar(80);

open qprint;
open(e) qprint;
line = %char(%error) + ' ' + %char(%status(qprint));
feod(e) qprint;
line += ' ' + %char(%error);
close(e) qprint;
line += ' ' + %char(%error);
open gn120f;
close gn120f;
close(e) gn120f;
line += ' ' + %char(%error) + ' ' + %char(%status(gn120f));
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
