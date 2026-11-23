**FREE
// Phase 2: READ right after READE reached end of file (GN58PF of
// native58, its rows in this order): a record, or end of file.
ctl-opt dftactgrp(*no);
dcl-f GN58PF keyed;
dcl-s kOrd zoned(3 : 0);
kOrd = 1;
setll kOrd GN58PF;
reade kOrd GN58PF;
dow not %eof(GN58PF);
  reade kOrd GN58PF;
enddo;
read GN58PF;
if %eof(GN58PF);
  say('READ EOF');
else;
  say('READ ' + %char(ORD) + ' ' + %char(LIN));
endif;
read GN58PF;
if %eof(GN58PF);
  say('READ2 EOF');
else;
  say('READ2 ' + %char(ORD) + ' ' + %char(LIN));
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
