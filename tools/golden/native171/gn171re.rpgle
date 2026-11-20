**FREE
// Stage 2, S2.171: READ(E) of a database file: a READ(E) loop with
// %ERROR and %EOF, and READ(E) of a USROPN file that is not open, in a
// MONITOR group (its OPEN, which IBM i requires, never runs).
// GN164F holds the rows 1 'aa', 2 'bb', 3 'cc' (native164). Captured on
// PUB400 (tools/golden/native171.txt n171_reade) for the native unit
// NREADE.
ctl-opt dftactgrp(*no);
dcl-f gn164f usage(*input) rename(gn164f : gn164r);
dcl-f gn164k usage(*input) usropn extfile('GN164F') extdesc('GN164F') rename(gn164f : gn164s);
dcl-ds r likerec(gn164r);
dcl-s c int(10);
dcl-s line varchar(80);

read(e) gn164f r;
dow not %eof(gn164f);
  c += 1;
  read(e) gn164f r;
enddo;
line = %char(c) + ' ' + %char(%error) + ' ' + %char(%eof(gn164f));
if c < 0;
  open gn164k;
endif;
monitor;
  read(e) gn164k;
  line += ' ' + %char(%error) + ' ' + %char(%status(gn164k));
on-error;
  line += ' caught ' + %char(%status);
endmon;
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
