**FREE
// Stage 2, S2.210: READ and UPDATE of a record with float, graphic
// (VARGRAPHIC) fields into program fields: a float
// field read and written back, the graphic field changed; the record
// read again. Captured on PUB400
// (tools/golden/native210.txt n210_floatgraphic) for the native unit
// NFLTGRF.
ctl-opt dftactgrp(*no) extbinint(*yes);
dcl-f gn210t usage(*update);
dcl-s line varchar(200);

read gn210t;
line = %char(id) + ' ' + %char(rate) + ' ' + %trim(note);
rate = rate / 4;
note = 'Bern';
update gn210r;
setll *start gn210t;
read gn210t;
line += '|' + %char(id) + ' ' + %char(rate) + ' ' + %trim(note);
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
