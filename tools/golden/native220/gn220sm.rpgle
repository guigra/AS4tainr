**FREE
// Phase 2: SND-MSG *ESCAPE inside a MONITOR group with ON-EXCP for its
// message ID, in the main procedure and in a subprocedure (fidelity.md:
// on 2026-11-20 the message went to the caller).
ctl-opt dftactgrp(*no);
dcl-pr sub;
end-pr;
say('start');
monitor;
  sub();
  say('sub returned');
on-excp 'CPF9898';
  say('caught from sub');
endmon;
monitor;
  snd-msg *escape %msg('CPF9898' : 'QCPFMSG' : 'main');
  say('after main');
on-excp 'CPF9898';
  say('caught in main');
endmon;
say('end');
*inlr = *on;

dcl-proc sub;
  monitor;
    snd-msg *escape %msg('CPF9898' : 'QCPFMSG' : 'sub');
    say('after sub');
  on-excp 'CPF9898';
    say('caught in sub');
  endmon;
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
