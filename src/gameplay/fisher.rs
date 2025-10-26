use owo_colors::Style;
use rand::seq::IndexedRandom;

/// Catálogo de todos os peixes possíveis classificados por raridade
pub struct FishCatalog {
    abyssals: Vec<String>,
    mythicals: Vec<String>,
    legendaries: Vec<String>,
    shiny: Vec<String>,
    epics: Vec<String>,
    rares: Vec<String>,
    commons: Vec<String>,
}

impl FishCatalog {
    pub fn new() -> Self {
        FishCatalog {
            // Peixes comuns: 50% de ocorrência
            commons: vec![
                String::from("sardinha"),
                String::from("atum"),
                String::from("dourada"),
                String::from("cação"),
                String::from("traíra"),
                String::from("arenque"),
                String::from("robalo"),
                String::from("tambaqui"),
                String::from("corvina"),
                String::from("cavala"),
                String::from("truta"),
                String::from("pescada"),
                String::from("pacu"),
                String::from("lambari"),
                String::from("merluza"),
                String::from("anchova"),
            ],
            // Peixes raros: 25% de ocorrência
            rares: vec![
                String::from("pirarucu"),
                String::from("tucunaré"),
                String::from("salmão"),
                String::from("bacalhau"),
                String::from("pintado"),
                String::from("betta"),
                String::from("bagre"),
                String::from("peixe-palhaço"),
                String::from("garoupa"),
                String::from("ouriço"),
                String::from("peixe-cirurgião"),
                String::from("peixe-borboleta"),
                String::from("piranha"),
            ],
            // Peixes épicos: 15% de ocorrência
            epics: vec![
                String::from("robalo-gigante"),
                String::from("peixe-pedra"),
                String::from("carpa"),
                String::from("poraquê"),
                String::from("peixe-voador"),
                String::from("baiacu"),
                String::from("peixe-lua"),
                String::from("água-viva"),
            ],
            // Peixes shiny: 6% de ocorrência
            shiny: vec![
                String::from("botia-beijadora"),
                String::from("peixe-filhote-de-diabo"),
                String::from("peixe-mão-vermelho"),
                String::from("peixe-anjo-rainha"),
                String::from("peixe-lanterna"),
                String::from("dragão-marinho-comum"),
                String::from("peixe-leão"),
                String::from("cavalo-marinho"),
            ],
            // Peixes míticos: 2.5% de ocorrência
            legendaries: vec![
                String::from("peixe-serra"),
                String::from("marlin-azul"),
                String::from("peixe-espada"),
                String::from("baleia-jubarte"),
                String::from("enguia-pelicano"),
                String::from("quimera"),
                String::from("dragão-marinho-folhado"),
            ],
            // Peixes míticos: 1% de ocorrência
            mythicals: vec![
                String::from("peixe-napoleão"),
                String::from("raia-jamanta"),
                String::from("polvo-de-anéis-azuis"),
                String::from("peixe-mandarim"),
                String::from("peixe-morcego-de-lábios-vermelhos"),
            ],
            // Peixes abissais: 0.5% de ocorrência
            abyssals: vec![
                String::from("peixe-bolha"),
                String::from("peixe-pescador"),
                String::from("peixe-olho-de-barril"),
                String::from("lula-vampira-do-inferno"),
                String::from("tubarão-duende"),
                String::from("tubarão-elefante"),
            ],
        }
    }
    /// Retorna o estilo para a string de um peixe de acordo com a raridade
    pub fn get_style_for_fish(&self, fish_name: &str) -> Style {
        if self.abyssals.iter().any(|f| f == fish_name) {
            Style::new().fg_rgb::<150, 0, 60>().bold()
        } else if self.mythicals.iter().any(|f| f == fish_name) {
            Style::new().fg_rgb::<255, 130, 60>().bold()
        } else if self.legendaries.iter().any(|f| f == fish_name) {
            Style::new().fg_rgb::<240, 200, 60>().bold()
        } else if self.shiny.iter().any(|f| f == fish_name) {
            Style::new().fg_rgb::<255, 80, 135>().bold()
        } else if self.epics.iter().any(|f| f == fish_name) {
            Style::new().fg_rgb::<160, 15, 230>().bold()
        } else if self.rares.iter().any(|f| f == fish_name) {
            Style::new().fg_rgb::<80, 150, 255>().bold()
        } else {
            Style::new().fg_rgb::<100, 255, 160>().bold()
        }
    }
    /// Retorna um "rank" de raridade para um peixe
    /// 0 = comum, 6 = abissal
    pub fn get_rarity_rank(&self, fish_name: &str) -> u8 {
        if self.abyssals.iter().any(|f| f == fish_name) {
            6
        } else if self.mythicals.iter().any(|f| f == fish_name) {
            5
        } else if self.legendaries.iter().any(|f| f == fish_name) {
            4
        } else if self.shiny.iter().any(|f| f == fish_name) {
            3
        } else if self.epics.iter().any(|f| f == fish_name) {
            2
        } else if self.rares.iter().any(|f| f == fish_name) {
            1
        } else {
            0
        }
    }
}

impl Default for FishCatalog {
    fn default() -> Self {
        Self::new()
    }
}

/// Função de pesca, retorna um peixe aleatório do catálogo com distribuição proporcional a raridade
pub fn fishing(fish_catalog: &FishCatalog) -> String {
    let mut rng = rand::rng();
    // Probabilidades: 50% comum, 25% raro, 15% épico, 6% shiny, 2.5% lendário, 1% mítico, 0.5% abissal
    let x: u32 = rand::random::<u32>() % 200;

    let fish_list = match x {
        0 => &fish_catalog.abyssals,
        1..=2 => &fish_catalog.mythicals,
        3..=7 => &fish_catalog.legendaries,
        8..=19 => &fish_catalog.shiny,
        20..=49 => &fish_catalog.epics,
        50..=99 => &fish_catalog.rares,
        _ => &fish_catalog.commons,
    };

    fish_list.choose(&mut rng).unwrap().clone()
}
