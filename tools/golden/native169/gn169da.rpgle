**FREE
// Stage 2, S2.169: a whole data structure array passed to a CONST
// LIKEDS DIM parameter, global and a procedure's local, and the
// procedure reading its elements' subfields by m(i).field. Captured on
// PUB400 (tools/golden/native169.txt n169_dsaparm) for the native unit
// NDSAPRM.
ctl-opt dftactgrp(*no);
dcl-ds m_t qualified template;
  parmNo int(10);
  val varchar(20);
end-ds;
dcl-ds arr likeds(m_t) dim(4) inz;
dcl-s line varchar(120);

arr(1).parmNo = 1;
arr(2).parmNo = 2;
arr(2).val = 'two';
arr(3).parmNo = 3;
line = show(arr : 3);
line += '|' + inner();
say(line);
*inlr = *on;

dcl-proc show;
  dcl-pi *n varchar(60);
    m likeds(m_t) dim(4) const;
    n int(10) const;
  end-pi;
  dcl-s i int(10);
  dcl-s o varchar(60);
  for i = 1 to n;
    o += %char(m(i).parmNo) + m(i).val + ' ';
  endfor;
  return o;
end-proc;

dcl-proc inner;
  dcl-pi *n varchar(60);
  end-pi;
  dcl-ds loc likeds(m_t) dim(4) inz;
  loc(1).parmNo = 9;
  loc(1).val = 'x';
  loc(4).parmNo = 4;
  loc(4).val = 'four';
  return show(loc : 4);
end-proc;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
