**FREE
// Stage 2, S2.77: a path with two indexes (gv(s).m(i).parmNo): a LIKEDS
// subfield with DIM inside a data structure array, written and read with
// literal and field indexes, an index that is itself such a path
// (gv(s).m(gv(s).nm)), and an inner index past its elements caught by
// MONITOR (status 121). Captured on PUB400 (tools/golden/native77.txt
// n77_nested2) for the native unit NNEST2.
ctl-opt dftactgrp(*no);
dcl-ds m_t qualified template;
  parmNo int(10);
  val char(8);
end-ds;
dcl-ds gv qualified dim(3);
  nm int(10);
  m likeds(m_t) dim(4);
end-ds;
dcl-s s int(10) inz(2);
dcl-s i int(10);
dcl-s line varchar(52);
gv(s).nm = 3;
for i = 1 to 3;
  gv(s).m(i).parmNo = i * 10;
  gv(s).m(i).val = 'v' + %char(i);
endfor;
gv(3).m(4).parmNo = gv(s).m(gv(s).nm).parmNo + gv(2).m(1).parmNo;
line = 'GV';
for i = 1 to gv(s).nm;
  line += ' ' + %char(gv(s).m(i).parmNo) + %trim(gv(s).m(i).val);
endfor;
say(line + ' L ' + %char(gv(3).m(4).parmNo) + ' ' + %char(gv(1).m(4).parmNo));
i = 5;
monitor;
  gv(s).m(i).parmNo = 1;
  say('NO ERROR');
on-error;
  say('ERROR ' + %char(%status));
endmon;
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
