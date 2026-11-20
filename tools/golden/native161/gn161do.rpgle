**FREE
// Stage 2, S2.161 (difference, not a unit): OPEN(E) of a database file
// that is already open: %ERROR and %STATUS on IBM i, where the
// interpreter raises CPF4161 (fidelity.md). Captured on PUB400
// (tools/golden/native161.txt n161_dblopen).
ctl-opt dftactgrp(*no);
dcl-f gn120f usage(*input) usropn rename(gn120f : gn120r);
dcl-s line varchar(80);

open gn120f;
open(e) gn120f;
line = %char(%error) + ' ' + %char(%status(gn120f));
close gn120f;
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
