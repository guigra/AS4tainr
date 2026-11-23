**FREE
// Phase 2: date and time literals in calculations under DATFMT(*EUR)
// TIMFMT(*USA): the DATFMT/TIMFMT forms, compared and assigned.
ctl-opt dftactgrp(*no) datfmt(*eur) timfmt(*usa);
dcl-s d date;
dcl-s t time;
d = d'31.01.2026';
t = t'09:15 AM';
if d = d'31.01.2026';
  say('EQ ' + %char(d) + ' ' + %char(t));
endif;
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
