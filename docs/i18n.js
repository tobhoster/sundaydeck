(() => {
  "use strict";

  const COPY = {
    en: {
      "nav.film": "Film",
      "nav.story": "Sunday Morning",
      "nav.global": "Every Tradition",
      "nav.try": "Try It Live",
      "nav.packs": "Starter Packs",
      "nav.sheet": "Cheat Sheet",
      "nav.download": "Download",
      "hero.kicker1": "Free & Open Source",
      "hero.kicker2": "No Licenses · Mac, Windows & Linux",
      "hero.h1": "Calm church slides built for <span class=\"serif-accent\">real Sunday volunteers.</span>",
      "hero.lead": "SundayDeck is free, lightweight presentation software made so any volunteer can sit down ten minutes before service and run worship lyrics, Scripture, sermon slides, and livestream lower-thirds with quiet confidence — no seat licenses, no subscriptions, no internet required.",
      "hero.ctaDownload": "Download Free",
      "hero.ctaFilm": "Watch the Film",
      "hero.ctaStory": "Walk through Sunday morning",
      "hero.meta1": "100% Offline & Private",
      "hero.meta2": "Sanctuary, Stage & Livestream Ready",
      "hero.meta3": "Zero Licenses · Zero Subscriptions",
      "film.label": "One-Minute Overview",
      "film.h2": "From pastor's notes to full sanctuary screens before the doors open.",
      "film.copy": "Watch how SundayDeck moves calmly from a pastor's sermon notes to custom slide layouts, peaceful photography and motion backgrounds, hands-free microphone auto-follow, and bilingual worship across North and South America.",
      "story.label": "A Sunday Morning Story",
      "story.h2": "Follow a volunteer from <span class=\"serif-accent\">9:45 AM to benediction.</span>",
      "story.copy": "Scroll through the morning — or click any moment — to see how SundayDeck stays calm, predictable, and out of your way when real services change on the fly.",
      "ch1.time": "9:45 AM · Fifteen minutes before service",
      "ch1.h3": "The pastor sends over the sermon notes.",
      "ch1.p": "Drop a PowerPoint deck, Word document, or plain outline straight into SundayDeck. Songs link to your library, Scriptures pull cleanly offline, and every item waits in a clear checklist for your approval before going into the service order.",
      "ch2.time": "9:52 AM · One last-minute announcement",
      "ch2.h3": "Create clean slides without opening PowerPoint.",
      "ch2.p": "Pick a curated layout — welcome title, key quote, click-to-reveal sermon bullets, photo + text announcement, or pre-service countdown — and choose a calm slide transition while your sanctuary backdrop stays steady behind the words.",
      "ch3.time": "10:00 AM · Worship begins",
      "ch3.h3": "Warm visuals for the room, clarity for the stage.",
      "ch3.p": "Project lyrics over peaceful photography or gentle looping backgrounds while your stage monitor shows the worship team the current line, the next slide, and a quiet countdown — and your livestream receives crisp transparent lower-thirds automatically.",
      "ch4.time": "10:20 AM · Spontaneous chorus & preaching",
      "ch4.h3": "Your microphone keeps pace — completely offline.",
      "ch4.p": "Turn on microphone auto-follow in safe Suggest or Auto mode so slides stay with the worship leader even when they circle back to the chorus. During the sermon, spoken Bible verses appear in a calm one-click queue without ever hijacking the screen.",
      "ch5.time": "10:45 AM · One church, every generation & language",
      "ch5.h3": "Bilingual lyrics, hymn numbers, and responsive readings.",
      "ch5.p": "Jump directly to hymns by book and number, display English alongside Spanish or Portuguese on one clean slide, or project Leader and Congregation call-and-response readings with clear visual contrast from the back pew.",
      "global.label": "Across North & South America",
      "global.h2": "Contemporary, Baptist, Catholic, Mainline, and Bilingual churches — ready out of the box.",
      "global.copy": "SundayDeck welcomes churches across the Americas with Protestant and Catholic Bible canons, instant hymnbook number lookup, native bilingual slides, and easy migration from your existing song library.",
      "try.label": "Try the Booth Controls",
      "try.h2": "Feel how calm recovery keys are when plans change live.",
      "try.copy": "Click any control below to test what the sanctuary screen does — advancing slides, hiding words while keeping your background steady, fading gently to black, or bringing up your church logo in one tap.",
      "install.label": "Download SundayDeck",
      "install.h2": "Free installers for Mac, Windows, and Linux.",
      "install.copy": "No account sign-up, no seat licenses, and no watermark. Starts in under a second even on modest older church laptops.",
      "conv.label": "Our Conviction",
      "conv.h2": "Church software should be <span class=\"serif-accent\">free and fully accessible.</span>",
      "conv.copy": "No seat licenses. No annual renewals. No locked features, watermarks, or forced upgrades. Whether you are a small church plant meeting in a living room or a busy Sunday sanctuary, every feature in SundayDeck is free forever — and we warmly welcome contributions from the community.",
      "sup.kicker": "Join & Support the Movement (Optional)",
      "sup.h3": "Keep SundayDeck free for every church.",
      "sup.p": "Every feature stays 100% free with zero paywalls, forever. If SundayDeck serves your congregation well, you can chip in a coffee or sponsor ongoing development — helping cover Mac and Windows code-signing certificates, release hosting, and starter packs for smaller churches.",
      "sup.coffee": "Buy Me a Coffee",
      "sup.sponsors": "GitHub Sponsors",
      "sup.kofi": "Support on Ko-fi",
      "sup.github": "Contribute on GitHub"
    },
    es: {
      "nav.film": "Video",
      "nav.story": "Domingo",
      "nav.global": "Iglesias",
      "nav.try": "Probar",
      "nav.packs": "Paquetes",
      "nav.sheet": "Guía 1 Página",
      "nav.download": "Descargar",
      "hero.kicker1": "Gratis y de Código Abierto",
      "hero.kicker2": "Sin Licencias · Mac, Windows y Linux",
      "hero.h1": "Proyección tranquila pensada para <span class=\"serif-accent\">voluntarios de domingo.</span>",
      "hero.lead": "SundayDeck es software gratuito y ligero de proyección para iglesias, creado para que cualquier voluntario pueda sentarse diez minutos antes del culto y proyectar alabanza, Biblia, sermones y subtítulos en vivo con total calma — sin licencias, sin suscripciones y sin internet.",
      "hero.ctaDownload": "Descargar Gratis",
      "hero.ctaFilm": "Ver el Video",
      "hero.ctaStory": "Recorre un domingo por la mañana",
      "hero.meta1": "100% Sin Internet y Privado",
      "hero.meta2": "Templo, Escenario y Transmisión",
      "hero.meta3": "Cero Licencias · Cero Suscripciones",
      "film.label": "Resumen en Un Minuto",
      "film.h2": "De las notas del pastor a todas las pantallas antes de abrir las puertas.",
      "film.copy": "Mira cómo SundayDeck transforma las notas del pastor en un orden de servicio claro, con diseños integrados, fondos tranquilos, seguimiento por micrófono sin internet y adoración bilingüe para toda América.",
      "story.label": "Historia de un Domingo",
      "story.h2": "Acompaña a un voluntario de <span class=\"serif-accent\">9:45 AM a la bendición final.</span>",
      "story.copy": "Desplázate por la mañana — o haz clic en cada momento — para ver cómo SundayDeck mantiene todo sereno y bajo control incluso cuando el servicio cambia en vivo.",
      "ch1.time": "9:45 AM · Quince minutos antes del culto",
      "ch1.h3": "El pastor envía las notas del sermón.",
      "ch1.p": "Arrastra una presentación de PowerPoint, un documento de Word o un bosquejo simple. SundayDeck enlaza tus cantos, busca los pasajes bíblicos sin internet y deja todo listo para revisar antes de pasar a pantalla.",
      "ch2.time": "9:52 AM · Un anuncio de último minuto",
      "ch2.h3": "Crea diapositivas limpias sin abrir PowerPoint.",
      "ch2.p": "Elige una plantilla clara — bienvenida, frase clave, puntos del sermón paso a paso, anuncio con foto o cuenta regresiva — con transiciones suaves mientras tu fondo permanece estable.",
      "ch3.time": "10:00 AM · Comienza la adoración",
      "ch3.h3": "Visuales serenos para el templo y claridad en el escenario.",
      "ch3.p": "Proyecta letras sobre fotografía tranquila o fondos en movimiento mientras el monitor de escenario muestra el verso actual, el siguiente y el temporizador — y tu transmisión en vivo recibe subtítulos transparentes al instante.",
      "ch4.time": "10:20 AM · Coros espontáneos y predicación",
      "ch4.h3": "Tu micrófono sigue el ritmo — totalmente sin internet.",
      "ch4.p": "Activa el seguimiento por micrófono en modo Sugerir o Automático para acompañar al equipo de alabanza incluso al repetir el coro. Durante la prédica, los versículos citados aparecen en una cola de un clic sin interrumpir la pantalla.",
      "ch5.time": "10:45 AM · Una iglesia, cada generación e idioma",
      "ch5.h3": "Letras bilingües, himnarios numerados y lecturas responsivas.",
      "ch5.p": "Salta al instante por número de himno, proyecta español e inglés en la misma diapositiva o presenta respuestas litúrgicas entre guía y congregación con excelente lectura desde la última fila.",
      "global.label": "En Norteamérica y Sudamérica",
      "global.h2": "Iglesias contemporáneas, bautistas, católicas, metodistas y bilingües — listas al instante.",
      "global.copy": "SundayDeck incluye Biblias protestantes de 66 libros y católicas de 73 libros, búsqueda sin preocuparse por tildes, himnarios numerados, letras bilingües e importación directa desde tu software anterior.",
      "try.label": "Prueba los Controles en Vivo",
      "try.h2": "Siente lo tranquilas que son las teclas de emergencia.",
      "try.copy": "Haz clic en cualquier botón para probar cómo responde la pantalla del templo: avanzar diapositivas, ocultar texto manteniendo el fondo, fundir a negro para oración o mostrar el logo de la iglesia.",
      "install.label": "Descargar SundayDeck",
      "install.h2": "Instaladores gratuitos para Mac, Windows y Linux.",
      "install.copy": "Sin cuentas, sin claves de licencia y sin marcas de agua. Abre en menos de un segundo incluso en computadoras antiguas de iglesia.",
      "conv.label": "Nuestra Convicción",
      "conv.h2": "El software para iglesias debe ser <span class=\"serif-accent\">gratuito y accesible para todos.</span>",
      "conv.copy": "Sin licencias por asiento. Sin renovaciones anuales. Sin funciones bloqueadas ni marcas de agua. Desde una iglesia pequeña hasta un templo grande, todo en SundayDeck es gratis para siempre — y recibimos con alegría tus contribuciones.",
      "sup.kicker": "Apoya el Movimiento (Opcional)",
      "sup.h3": "Ayuda a mantener SundayDeck gratis para cada iglesia.",
      "sup.p": "Cada función seguirá siendo 100% gratuita para siempre. Si SundayDeck bendice a tu congregación cada domingo, puedes invitar un café o patrocinar los certificados de firma para Mac y Windows y el desarrollo continuo.",
      "sup.coffee": "Invítame un Café",
      "sup.sponsors": "GitHub Sponsors",
      "sup.kofi": "Apoyar en Ko-fi",
      "sup.github": "Contribuir en GitHub"
    },
    pt: {
      "nav.film": "Vídeo",
      "nav.story": "Domingo",
      "nav.global": "Igrejas",
      "nav.try": "Testar",
      "nav.packs": "Pacotes",
      "nav.sheet": "Guia 1 Página",
      "nav.download": "Baixar",
      "hero.kicker1": "Gratuito e Código Aberto",
      "hero.kicker2": "Sem Licenças · Mac, Windows e Linux",
      "hero.h1": "Projeção tranquila feita para <span class=\"serif-accent\">voluntários de domingo.</span>",
      "hero.lead": "SundayDeck é um programa gratuito e leve de projeção para igrejas, criado para que qualquer voluntário possa sentar dez minutos antes do culto e operar letras, Bíblia, sermões e legendas de transmissão com calma — sem licenças, sem mensalidades e sem internet.",
      "hero.ctaDownload": "Baixar Grátis",
      "hero.ctaFilm": "Assistir ao Vídeo",
      "hero.ctaStory": "Acompanhe uma manhã de domingo",
      "hero.meta1": "100% Offline e Privado",
      "hero.meta2": "Templo, Palco e Transmissão",
      "hero.meta3": "Zero Licenças · Zero Assinaturas",
      "film.label": "Visão Geral em Um Minuto",
      "film.h2": "Das notas do pastor para todas as telas antes das portas abrirem.",
      "film.copy": "Veja como o SundayDeck transforma as notas do pastor em uma ordem de culto clara, com layouts prontos, fundos serenos, acompanhamento por microfone offline e louvor bilíngue nas Américas.",
      "story.label": "História de um Domingo",
      "story.h2": "Acompanhe um voluntário das <span class=\"serif-accent\">9h45 até a bênção final.</span>",
      "story.copy": "Role pela manhã — ou clique em qualquer momento — para ver como o SundayDeck mantém tudo sereno e previsível mesmo quando o culto muda ao vivo.",
      "ch1.time": "9:45 AM · Quinze minutos antes do culto",
      "ch1.h3": "O pastor envia as anotações da mensagem.",
      "ch1.p": "Arraste um PowerPoint, documento Word ou esboço simples. O SundayDeck conecta os hinos da sua biblioteca, busca os textos bíblicos offline e organiza tudo para revisão antes de ir para a tela.",
      "ch2.time": "9:52 AM · Um aviso de última hora",
      "ch2.h3": "Crie slides limpos sem abrir o PowerPoint.",
      "ch2.p": "Escolha um layout pronto — boas-vindas, frase-chave, tópicos da pregação passo a passo, aviso com imagem ou contagem regressiva — com transições suaves mantendo o fundo firme.",
      "ch3.time": "10:00 AM · O louvor começa",
      "ch3.h3": "Visual acolhedor no templo, clareza no palco.",
      "ch3.p": "Projete letras sobre fotos serenas ou fundos em movimento enquanto o monitor de palco mostra o slide atual, o próximo e o cronômetro — e sua transmissão ao vivo recebe legendas transparentes automaticamente.",
      "ch4.time": "10:20 AM · Refrão espontâneo e pregação",
      "ch4.h3": "Seu microfone acompanha o culto — totalmente offline.",
      "ch4.p": "Ative o acompanhamento por microfone no modo Sugerir ou Automático para seguir o louvor mesmo quando o ministro repete o refrão. Na pregação, versículos falados aguardam em uma fila de um clique sem trocar a tela sem aviso.",
      "ch5.time": "10:45 AM · Uma igreja, todas as gerações e idiomas",
      "ch5.h3": "Letras bilíngues, hinários numerados e leituras responsivas.",
      "ch5.p": "Pule direto pelo número do hino, projete português e inglês juntos em um único slide ou exiba leituras responsivas entre dirigente e congregação com ótima leitura até o fundo da igreja.",
      "global.label": "Nas Américas do Norte e do Sul",
      "global.h2": "Igrejas contemporâneas, batistas, católicas, metodistas e bilíngues — prontas para usar.",
      "global.copy": "O SundayDeck acolhe igrejas das Américas com Bíblias protestantes de 66 livros e católicas de 73 livros, busca sem se preocupar com acentos, hinários numerados e migração fácil da sua biblioteca atual.",
      "try.label": "Teste os Controles ao Vivo",
      "try.h2": "Sinta como as teclas de emergência são tranquilas ao vivo.",
      "try.copy": "Clique nos botões abaixo para testar a tela do templo: avançar slides, ocultar letras mantendo o fundo, escurecer suavemente para oração ou mostrar o logotipo da igreja.",
      "install.label": "Baixar o SundayDeck",
      "install.h2": "Instaladores gratuitos para Mac, Windows e Linux.",
      "install.copy": "Sem cadastro, sem chaves de licença e sem marca d'água. Abre em menos de um segundo mesmo em notebooks antigos da igreja.",
      "conv.label": "Nossa Convicção",
      "conv.h2": "Software para igrejas deve ser <span class=\"serif-accent\">gratuito e acessível a todos.</span>",
      "conv.copy": "Sem licenças por computador. Sem renovações anuais. Sem recursos bloqueados ou marcas d'água. De uma igreja pequena em início até um grande templo, tudo no SundayDeck é grátis para sempre — e contribuições são muito bem-vindas.",
      "sup.kicker": "Apoie o Movimento (Opcional)",
      "sup.h3": "Ajude a manter o SundayDeck gratuito para cada igreja.",
      "sup.p": "Cada recurso continuará 100% gratuito para sempre. Se o SundayDeck abençoa sua igreja aos domingos, você pode pagar um café ou apoiar certificados de assinatura para Mac/Windows e a manutenção contínua.",
      "sup.coffee": "Pague um Café",
      "sup.sponsors": "GitHub Sponsors",
      "sup.kofi": "Apoiar no Ko-fi",
      "sup.github": "Contribuir no GitHub"
    }
  };

  const buttons = Array.from(document.querySelectorAll(".lang-btn[data-lang]"));
  if (!buttons.length) return;

  function applyLang(lang) {
    const dict = COPY[lang] || COPY.en;
    document.documentElement.lang = lang;
    document.querySelectorAll("[data-i18n]").forEach((el) => {
      const key = el.getAttribute("data-i18n");
      if (dict[key]) el.textContent = dict[key];
    });
    document.querySelectorAll("[data-i18n-html]").forEach((el) => {
      const key = el.getAttribute("data-i18n-html");
      if (dict[key]) el.innerHTML = dict[key];
    });
    buttons.forEach((btn) => {
      btn.classList.toggle("is-active", btn.dataset.lang === lang);
      btn.setAttribute("aria-pressed", btn.dataset.lang === lang ? "true" : "false");
    });
    try {
      localStorage.setItem("sundaydeck_lang", lang);
    } catch (_) {}
  }

  buttons.forEach((btn) => {
    btn.addEventListener("click", () => applyLang(btn.dataset.lang));
  });

  let initial = "en";
  try {
    const saved = localStorage.getItem("sundaydeck_lang");
    if (saved && COPY[saved]) {
      initial = saved;
    } else if (navigator.language) {
      const lower = navigator.language.toLowerCase();
      if (lower.startsWith("es")) initial = "es";
      else if (lower.startsWith("pt")) initial = "pt";
    }
  } catch (_) {}
  applyLang(initial);
})();
