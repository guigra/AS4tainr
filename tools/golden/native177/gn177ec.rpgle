**FREE
// Stage 2, S2.177: EVAL-CORR: corresponding subfields by name and type
// class (character into varying, packed into zoned), a data structure
// subfield in both (its own subfields), arrays up to the smaller one's
// elements, character against packed and a target-only subfield kept.
// Captured on PUB400 (tools/golden/native177.txt n177_evalcorr) for the
// native unit NEVCORR.
ctl-opt dftactgrp(*no);
dcl-ds addr_t qualified template;
  city char(10);
  zip packed(5:0);
end-ds;
dcl-ds a qualified;
  name char(10) inz('Alice');
  age packed(3:0) inz(30);
  code char(3) inz('ABC');
  where likeds(addr_t);
  tags char(2) dim(3);
end-ds;
dcl-ds b qualified;
  age zoned(5:0) inz(1);
  name varchar(20) inz('x');
  code packed(3:0) inz(7);
  where likeds(addr_t);
  tags char(2) dim(2);
  oth char(1) inz('O');
end-ds;
dcl-s line varchar(120);

a.where.city = 'Paris';
a.where.zip = 75001;
a.tags(1) = 'T1';
a.tags(2) = 'T2';
a.tags(3) = 'T3';
eval-corr b = a;
line = '[' + b.name + '] ' + %char(b.age) + ' ' + %char(b.code) + ' '
     + b.where.city + %char(b.where.zip) + ' ' + b.tags(1) + b.tags(2)
     + ' ' + b.oth;
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
