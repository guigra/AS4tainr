**FREE
// OVERLAY(field : *NEXT) after a group subfield, for the review of
// upstream PR #14.
ctl-opt dftactgrp(*no);
dcl-ds grpds;
  pn char(10);
  grp overlay(pn);
    g1 char(2) overlay(grp);
    g2 char(2) overlay(grp : *next);
  gc char(3) overlay(pn : *next);
end-ds;
pn = 'ABCDEFGHIJ';
say('GRP G1=' + g1 + '|G2=' + g2 + '|GRP=' + grp + '|GC=' + gc + '|');
say('GRP SIZE=' + %char(%size(grpds)));
*inlr = *on;
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
