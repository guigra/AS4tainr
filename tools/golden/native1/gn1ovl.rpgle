**FREE
// OVERLAY(field : *NEXT): where the subfields go.
ctl-opt dftactgrp(*no);
dcl-ds part;
  PartNumber char(10);
  Family char(3) overlay(PartNumber);
  Sequence char(6) overlay(PartNumber : *next);
  Language char(1) overlay(PartNumber : *next);
end-ds;
PartNumber = 'ABC123456E';
say('OVL FAMILY=' + Family + '|SEQUENCE=' + Sequence + '|LANGUAGE=' + Language + '|');
say('OVL SIZE=' + %char(%size(part)));
*inlr = *on;
// Write one line to gn1.out in the job's current directory (DSPLY under
// `system` goes to QSYSOPR, a queue PUB400 users share).
dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s line varchar(200);
  line = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :line,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
