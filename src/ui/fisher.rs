use owo_colors::Style;
use rand::seq::IndexedRandom;

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
                String::from("peixe-mão vermelho"),
                String::from("peixe-anjo-rainha"),
                String::from("peixe-lanterna"),
                String::from("dragão-marinho-comum"),
                String::from("peixe-leão"),
                String::from("cavalo-marinho"),
            ],
            // Peixes míticos: 2.5% de ocorrência
            legendaries: vec![
                String::from("peixe-serra"),
                String::from("marlin azul"),
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
                String::from("peixe-morcego de lábios vermelhos"),
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
}

#[allow(unused_variables)]
pub fn fishing(fish_catalog: &FishCatalog) -> String {
    let fish = {
        let mut rng = rand::rng();
        let x = rand::random_range(1..=200);
        match x {
            1 => {
                let fish = fish_catalog.abyssals.choose(&mut rng).unwrap();
                Style::new().fg_rgb::<150, 0, 60>().bold().style(fish)
            }
            2..=3 => {
                let fish = fish_catalog.mythicals.choose(&mut rng).unwrap();
                Style::new().fg_rgb::<255, 130, 60>().bold().style(fish)
            }
            4..=8 => {
                let fish = fish_catalog.legendaries.choose(&mut rng).unwrap();
                Style::new().fg_rgb::<240, 200, 60>().bold().style(fish)
            }
            9..=20 => {
                let fish = fish_catalog.shiny.choose(&mut rng).unwrap();
                Style::new().fg_rgb::<255, 80, 135>().bold().style(fish)
            }
            21..=50 => {
                let fish = fish_catalog.epics.choose(&mut rng).unwrap();
                Style::new().fg_rgb::<160, 15, 230>().bold().style(fish)
            }
            61..=100 => {
                let fish = fish_catalog.rares.choose(&mut rng).unwrap();
                Style::new().fg_rgb::<80, 150, 255>().bold().style(fish)
            }
            _ => {
                let fish = fish_catalog.commons.choose(&mut rng).unwrap();
                Style::new().fg_rgb::<100, 255, 160>().bold().style(fish)
            }
        }
    };
    format!("Você pescou um(a) {}!\n", fish)
}

