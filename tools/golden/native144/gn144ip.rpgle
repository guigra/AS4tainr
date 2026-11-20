**FREE
// Stage 2, S2.144: indicator subfields of data structure array elements
// by their path: in a concatenation, assigned, and in IF and NOT, with
// literal and field indexes. Captured on PUB400
// (tools/golden/native144.txt n144_indpath) for the native unit NINDPATH.
ctl-opt dftactgrp(*no);
dcl-ds val_t qualified template;
  isNull ind inz(*on);
  text varchar(10) inz;
end-ds;
dcl-ds list qualified;
  ev likeds(val_t) dim(3) inz(*likeds);
end-ds;
dcl-s i int(10) inz(2);
dcl-s line varchar(80);

line = list.ev(1).isNull + list.ev(i).isNull;
list.ev(2).isNull = *off;
line += list.ev(i).isNull;
if list.ev(3).isNull;
  line += 'y';
endif;
if not list.ev(i).isNull;
  line += 'n';
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
